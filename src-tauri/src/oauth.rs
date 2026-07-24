//! "Login with Atlassian" — OAuth 2.0 (3LO) cho Jira Cloud.
//!
//! Atlassian 3LO KHONG ho tro PKCE/public client: doi code lay token bat buoc
//! co `client_secret`. Desktop app khong giau duoc secret trong binary, nen
//! secret nam o backend nho (backend/oauth-proxy — Cloudflare Worker) va app
//! chi noi chuyen voi backend do khi can DOI/LAM MOI token. Moi request Jira
//! van di thang toi Atlassian bang access token.
//!
//! Phan chia bi mat:
//!   - client_secret: CHI o backend, app khong bao gio thay
//!   - refresh token: Keychain (`jira-widget/jira-oauth`) — song lau, XOAY VONG
//!   - access token:  RAM (TokenStore), song ~1h, khong bao gio ghi ra dia/log
//!
//! Luong dang nhap: mo browser toi trang consent cua Atlassian; callback ve
//! backend roi backend 302 tiep ve cong loopback 127.0.0.1 ngau nhien cua app
//! (tham so `state = nonce.port` de backend biet cong nao va app biet request
//! that). Nhan code -> doi token qua backend -> luu refresh -> hoi danh sach
//! site (accessible-resources) -> nguoi dung chon site -> xong.

use crate::config::{self, Config};
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::Instant;

const AUTHORIZE_URL: &str = "https://auth.atlassian.com/authorize";
const RESOURCES_URL: &str = "https://api.atlassian.com/oauth/token/accessible-resources";
const API_BASE: &str = "https://api.atlassian.com/ex/jira";
/// Classic scope: du cho REST platform + agile (board/sprint/issue) o che do doc.
/// `offline_access` bat buoc — thieu no Atlassian khong cap refresh token.
const SCOPES: &str = "read:jira-work read:jira-user offline_access";
/// Access token con it hon ngan nay thi refresh truoc khi dung.
const EXPIRY_MARGIN: Duration = Duration::from_secs(60);
/// Cho nguoi dung bam consent tren browser toi da bay nhieu.
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(180);
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

/// API base cho mot site Cloud: moi request REST di qua gateway cua Atlassian.
pub fn api_base_for(cloud_id: &str) -> String {
    format!("{API_BASE}/{}", cloud_id.trim())
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
}

/// Mot site Jira Cloud ma tai khoan nay vao duoc.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSite {
    pub id: String,
    pub url: String,
    pub name: String,
}

// ------------------------------------------------------------- TokenStore

/// Store OAuth dung CHUNG cho toan process.
///
/// Refresh token cua Atlassian XOAY VONG (moi cai dung dung MOT lan): hai
/// instance rieng le cung refresh la mot ben cam token da bi thu hoi — poller
/// se chet "token het han" oan ~1h sau khi mo Cai dat. Vi vay MOI noi can
/// OAuth (client chinh luc khoi dong, probe cua Settings/wizard, whoami) deu
/// phai lay store qua ham nay, khong duoc tu dung instance rieng.
static SHARED: std::sync::OnceLock<std::sync::Mutex<Option<std::sync::Arc<TokenStore>>>> =
    std::sync::OnceLock::new();

pub fn shared_store(cfg: &Config) -> Result<std::sync::Arc<TokenStore>> {
    let cell = SHARED.get_or_init(|| std::sync::Mutex::new(None));
    let mut g = cell.lock().expect("SHARED mutex poisoned");
    if let Some(s) = g.as_ref() {
        return Ok(s.clone());
    }
    let s = std::sync::Arc::new(TokenStore::from_keychain(cfg)?);
    *g = Some(s.clone());
    Ok(s)
}

/// Giu access token hien hanh va lo viec refresh.
///
/// Mutex serialize CA doc lan refresh: hai request cung dinh 401 mot luc ma
/// refresh dua nhau thi mot ben se cam refresh token DA BI THU HOI (Atlassian
/// xoay vong refresh token) — phien dang nhap chet ngay tai cho. Sau khi giu
/// khoa phai kiem tra lai: co the ben kia vua refresh xong roi.
pub struct TokenStore {
    http: reqwest::Client,
    backend: String,
    inner: Mutex<TokenInner>,
}

struct TokenInner {
    access_token: String,
    expires_at: Instant,
    refresh_token: String,
}

impl TokenStore {
    /// Dung tu refresh token dang nam trong Keychain (luc app khoi dong).
    /// Chua dang nhap thi van tra ve store rong — request dau tien se bao
    /// `Auth` va panel hien dung state "token het han / chua dang nhap".
    /// PRIVATE co chu y: ben ngoai phai di qua `shared_store`.
    fn from_keychain(cfg: &Config) -> Result<Self> {
        let backend = cfg
            .oauth_backend_url()
            .ok_or_else(|| anyhow!("chua cau hinh oauth_backend_url"))?;
        let refresh =
            config::keychain_get_account(config::KEYCHAIN_OAUTH_ACCOUNT).unwrap_or_default();
        Self::new(backend, String::new(), 0, refresh)
    }

    pub fn new(
        backend: String,
        access_token: String,
        expires_in: u64,
        refresh_token: String,
    ) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .user_agent("master-jira/0.1")
            .build()?;
        Ok(Self {
            http,
            backend: backend.trim_end_matches('/').to_string(),
            inner: Mutex::new(TokenInner {
                expires_at: Instant::now() + Duration::from_secs(expires_in),
                access_token,
                refresh_token,
            }),
        })
    }

    /// Access token con han; het thi tu refresh. Loi tra ve chuoi de client
    /// boc thanh `JiraError::Auth`.
    pub async fn access_token(&self) -> Result<String, String> {
        let mut g = self.inner.lock().await;
        if !g.access_token.is_empty()
            && g.expires_at.saturating_duration_since(Instant::now()) > EXPIRY_MARGIN
        {
            return Ok(g.access_token.clone());
        }
        self.refresh_locked(&mut g).await
    }

    /// Ep refresh sau khi dinh 401 — nhung CHI khi access token minh vua dung
    /// van dang la ban hien hanh. Khac di nghia la mot request khac vua refresh
    /// xong trong luc minh cho khoa: dung ban moi, khong dot them mot luot
    /// xoay vong. So theo GIA TRI token chu khong theo han: token bi thu hoi
    /// phia server (chua het han) van phai refresh duoc.
    pub async fn force_refresh_if_stale(&self, seen_access: &str) -> Result<(), String> {
        let mut g = self.inner.lock().await;
        if !g.access_token.is_empty() && g.access_token != seen_access {
            return Ok(());
        }
        self.refresh_locked(&mut g).await.map(|_| ())
    }

    /// Nap bo token vua doi duoc tu login vao store — giu nguyen Arc ma client
    /// chinh dang cam, nen poller dung ban moi ngay khong can khoi dong lai.
    pub async fn install(&self, access: String, expires_in: u64, refresh: String) {
        let mut g = self.inner.lock().await;
        g.access_token = access;
        g.expires_at = Instant::now() + Duration::from_secs(expires_in);
        g.refresh_token = refresh;
    }

    /// Xoa sach token trong RAM (dung khi logout).
    pub async fn clear(&self) {
        let mut g = self.inner.lock().await;
        g.access_token.clear();
        g.refresh_token.clear();
        g.expires_at = Instant::now();
    }

    async fn refresh_locked(&self, g: &mut TokenInner) -> Result<String, String> {
        if g.refresh_token.is_empty() {
            return Err("chua dang nhap Atlassian".into());
        }
        let resp = self
            .http
            .post(format!("{}/oauth/token", self.backend))
            .json(&serde_json::json!({
                "grant_type": "refresh_token",
                "refresh_token": g.refresh_token,
            }))
            .send()
            .await
            .map_err(|e| format!("khong toi duoc backend OAuth: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            let short: String = body.chars().take(160).collect();
            return Err(format!("refresh that bai (HTTP {}): {short}", status.as_u16()));
        }
        let tok: TokenResponse = resp
            .json()
            .await
            .map_err(|e| format!("phan hoi refresh khong doc duoc: {e}"))?;

        g.access_token = tok.access_token.clone();
        g.expires_at = Instant::now() + Duration::from_secs(tok.expires_in.unwrap_or(3600));

        // Atlassian XOAY VONG refresh token: ban moi thay ban cu, ban cu bi thu
        // hoi. Phai ghi de Keychain NGAY — mat ban moi la phien SAU khi thoat
        // app se chet (RAM van chay dung toi luc do). Ghi hong thi thu lai mot
        // lan roi keu to; khong chan request dang chay vi RAM van co token dung.
        if let Some(rt) = tok.refresh_token.filter(|s| !s.is_empty()) {
            g.refresh_token = rt.clone();
            if let Err(e) = config::keychain_set_account(config::KEYCHAIN_OAUTH_ACCOUNT, &rt) {
                log::warn!("khong luu duoc refresh token moi vao Keychain: {e} — thu lai");
                if let Err(e2) = config::keychain_set_account(config::KEYCHAIN_OAUTH_ACCOUNT, &rt)
                {
                    log::error!(
                        "van khong luu duoc refresh token moi ({e2}) — dang nhap Atlassian \
                         se mat sau khi thoat app, can dang nhap lai"
                    );
                }
            }
        }
        log::info!("da lam moi access token OAuth");
        Ok(g.access_token.clone())
    }
}

/// Dang xuat: xoa refresh token khoi Keychain VA khoi store dung chung —
/// khong xoa RAM thi poller van tu refresh duoc them ~1h nua sau khi logout.
pub async fn logout() -> Result<()> {
    config::keychain_delete_account(config::KEYCHAIN_OAUTH_ACCOUNT)?;
    let store = SHARED
        .get()
        .and_then(|cell| cell.lock().expect("SHARED mutex poisoned").clone());
    if let Some(s) = store {
        s.clear().await;
    }
    Ok(())
}

/// Da co refresh token trong Keychain chua — dung cho kiem tra first-run.
pub fn has_refresh_token() -> bool {
    config::keychain_get_account(config::KEYCHAIN_OAUTH_ACCOUNT).is_some()
}

// ------------------------------------------------------------ login flow

/// Ket qua dang nhap: danh sach site de nguoi dung chon (1 site thi UI tu chon).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResult {
    pub sites: Vec<CloudSite>,
}

fn nonce_hex() -> Result<String> {
    let mut buf = [0u8; 16];
    getrandom::getrandom(&mut buf)
        .map_err(|e| anyhow!("khong lay duoc random cho state: {e}"))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

/// Chay tron luong dang nhap. Tra ve danh sach site; refresh token da nam
/// trong Keychain khi ham nay tra Ok.
///
/// `open_url` duoc tiem vao de ham nay khong dinh vao AppHandle — test duoc.
pub async fn login(cfg: &Config, open_url: impl FnOnce(String) -> Result<()>) -> Result<LoginResult> {
    let client_id = cfg
        .oauth_client_id()
        .ok_or_else(|| anyhow!("Chua cau hinh OAuth client id"))?;
    let backend = cfg
        .oauth_backend_url()
        .ok_or_else(|| anyhow!("Chua cau hinh OAuth backend"))?;

    // Cong loopback ngau nhien, CHI bind 127.0.0.1 — khong nhan gi tu mang ngoai.
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .context("khong mo duoc cong loopback")?;
    let port = listener.local_addr()?.port();
    let nonce = nonce_hex()?;
    let state = format!("{nonce}.{port}");

    let redirect_uri = format!("{backend}/oauth/callback");
    let auth_url = reqwest::Url::parse_with_params(
        AUTHORIZE_URL,
        &[
            ("audience", "api.atlassian.com"),
            ("client_id", client_id.as_str()),
            ("scope", SCOPES),
            ("redirect_uri", redirect_uri.as_str()),
            ("state", state.as_str()),
            ("response_type", "code"),
            ("prompt", "consent"),
        ],
    )
    .context("khong dung duoc URL authorize")?;

    open_url(auth_url.to_string())?;

    let code = wait_for_code(listener, &state).await?;

    // Doi code lay token — qua backend vi chi backend co client_secret.
    let http = reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .user_agent("master-jira/0.1")
        .build()?;
    let resp = http
        .post(format!("{backend}/oauth/token"))
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": redirect_uri,
        }))
        .send()
        .await
        .context("khong toi duoc backend OAuth")?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let short: String = body.chars().take(160).collect();
        return Err(anyhow!("doi code lay token that bai (HTTP {}): {short}", status.as_u16()));
    }
    let tok: TokenResponse = resp.json().await.context("phan hoi token khong doc duoc")?;
    let refresh = tok
        .refresh_token
        .clone()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            anyhow!("Atlassian khong tra refresh token — app 3LO thieu scope offline_access")
        })?;
    config::keychain_set_account(config::KEYCHAIN_OAUTH_ACCOUNT, &refresh)
        .context("khong luu duoc refresh token vao Keychain")?;

    // Nap bo token moi vao store dung chung NGAY: (1) poller/probe dung duoc
    // access nay luon, khong dot them mot luot refresh (moi luot la mot lan
    // xoay vong), (2) store cu (neu co) dang cam refresh token da chet sau
    // lan dang nhap lai nay.
    match shared_store(cfg) {
        Ok(store) => {
            store
                .install(
                    tok.access_token.clone(),
                    tok.expires_in.unwrap_or(3600),
                    refresh.clone(),
                )
                .await;
        }
        Err(e) => log::warn!("khong nap duoc token vao shared store: {e:#}"),
    }

    // Site nao tai khoan nay vao duoc — de UI chon cloud_id.
    let sites: Vec<CloudSite> = http
        .get(RESOURCES_URL)
        .bearer_auth(&tok.access_token)
        .header("Accept", "application/json")
        .send()
        .await
        .context("khong lay duoc danh sach site")?
        .error_for_status()
        .context("accessible-resources tra loi")?
        .json()
        .await
        .context("danh sach site khong doc duoc")?;

    if sites.is_empty() {
        return Err(anyhow!(
            "Tai khoan nay khong co site Jira Cloud nao (hoac app 3LO chua duoc cap scope Jira)"
        ));
    }
    log::info!("dang nhap Atlassian OK — {} site", sites.len());
    Ok(LoginResult { sites })
}

/// Nhan dung MOT callback hop le tren cong loopback (bo qua request rac nhu
/// favicon), co han chot tong the.
async fn wait_for_code(listener: TcpListener, expected_state: &str) -> Result<String> {
    let deadline = Instant::now() + CALLBACK_TIMEOUT;
    loop {
        let (mut sock, _) = tokio::time::timeout_at(deadline, listener.accept())
            .await
            .map_err(|_| anyhow!("Het gio cho dang nhap (3 phut). Thu lai."))?
            .context("cong loopback hong")?;

        let mut buf = vec![0u8; 8192];
        let n = match tokio::time::timeout(Duration::from_secs(5), sock.read(&mut buf)).await {
            Ok(Ok(n)) => n,
            _ => 0,
        };
        let req = String::from_utf8_lossy(&buf[..n]);
        let path = req
            .lines()
            .next()
            .unwrap_or("")
            .split_whitespace()
            .nth(1)
            .unwrap_or("");

        let (code, state) = parse_cb_query(path);
        let hop_le = matches!((&code, &state), (Some(c), Some(s))
            if !c.is_empty() && s == expected_state);

        let body = if hop_le {
            "<h2>Đăng nhập thành công ✓</h2><p>Quay lại app Master Jira để tiếp tục.</p>"
        } else {
            "<h2>Đăng nhập không thành công</h2><p>Quay lại app Master Jira và thử lại.</p>"
        };
        let page = format!(
            "<!doctype html><meta charset=\"utf-8\"><title>Master Jira</title>\
             <body style=\"font-family:system-ui;display:grid;place-items:center;height:90vh;text-align:center\">\
             <div>{body}</div></body>"
        );
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{page}",
            page.len()
        );
        let _ = sock.write_all(resp.as_bytes()).await;
        let _ = sock.shutdown().await;

        if hop_le {
            return Ok(code.unwrap());
        }
        // Request khong khop (favicon, state la...) -> tiep tuc cho toi deadline.
    }
}

/// Parse `/cb?code=..&state=..` bang Url that de percent-decoding dung chuan.
fn parse_cb_query(path: &str) -> (Option<String>, Option<String>) {
    let Ok(url) = reqwest::Url::parse(&format!("http://127.0.0.1{path}")) else {
        return (None, None);
    };
    if url.path() != "/cb" {
        return (None, None);
    }
    let mut code = None;
    let mut state = None;
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "state" => state = Some(v.into_owned()),
            _ => {}
        }
    }
    (code, state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_base_ghep_dung_cloud_id() {
        assert_eq!(
            api_base_for(" abc-123 "),
            "https://api.atlassian.com/ex/jira/abc-123"
        );
    }

    #[test]
    fn parse_cb_query_doc_duoc_percent_encoding() {
        let (c, s) = parse_cb_query("/cb?code=ey%2Fabc&state=deadbeef.49152");
        assert_eq!(c.unwrap(), "ey/abc");
        assert_eq!(s.unwrap(), "deadbeef.49152");
    }

    #[test]
    fn parse_cb_query_sai_path_thi_bo() {
        assert_eq!(parse_cb_query("/favicon.ico"), (None, None));
        assert_eq!(parse_cb_query("khong-phai-path"), (None, None));
    }

    #[test]
    fn nonce_du_dai_va_khac_nhau() {
        let a = nonce_hex().unwrap();
        let b = nonce_hex().unwrap();
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn khong_ep_refresh_khi_nguoi_khac_vua_lam_moi() {
        // Request cu cam "tok-cu" dinh 401 dung luc mot request khac da refresh
        // ra "tok-moi" — khong duoc dot them mot luot xoay vong (khong cham mang:
        // duong refresh se fail neu bi goi vi backend la dia chi gia).
        let s = TokenStore::new(
            "https://proxy.example.invalid".into(),
            "tok-moi".into(),
            3600,
            "r1".into(),
        )
        .unwrap();
        s.force_refresh_if_stale("tok-cu").await.unwrap();
        assert_eq!(s.access_token().await.unwrap(), "tok-moi");
    }

    #[tokio::test]
    async fn install_va_clear_doi_trang_thai_dang_nhap() {
        // Store rong / da clear thi access_token phai Err "chua dang nhap"
        // (khong cham mang — refresh khong duoc goi khi refresh_token rong).
        let s = TokenStore::new("https://proxy.example.invalid".into(), String::new(), 0, String::new())
            .unwrap();
        assert!(s.access_token().await.is_err());
        s.install("a".into(), 3600, "r".into()).await;
        assert_eq!(s.access_token().await.unwrap(), "a");
        s.clear().await;
        assert!(s.access_token().await.is_err());
    }
}
