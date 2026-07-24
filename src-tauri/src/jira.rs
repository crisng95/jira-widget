//! Jira Server/DC client.
//!
//! Day la NOI DUY NHAT trong app cham vao PAT va cham vao mang (AC-5).
//! Webview khong bao gio thay token vi no chi nhan `SprintSnapshot` da tinh san.
//!
//! Dung Agile API scope theo board thay vi JQL `sprint in openSprints()`:
//! mot project thuong co nhieu board, va JQL kia khong gioi han theo board nao.
//! Neu hien chi mot board dang mo sprint thi no *tinh co* dung — roi sai am tham
//! dung ngay board thu hai mo sprint. (AC-2)

use anyhow::Result;
use chrono::{DateTime, FixedOffset, Utc};
use serde::Deserialize;
use std::fmt;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const SPRINT_CACHE_TTL: Duration = Duration::from_secs(600); // 10 phut (AC-3)
const PAGE_SIZE: u32 = 50;
const HTTP_TIMEOUT: Duration = Duration::from_secs(20);

/// Field can lay. Giu dung minimum de payload nho.
const FIELDS: &str = "key,summary,status,assignee,created,updated,issuetype,priority,\
customfield_10107,customfield_10704,customfield_10200,customfield_10201";

// ------------------------------------------------------------------ errors

#[derive(Debug, Clone)]
pub enum JiraError {
    /// 401/403 — token het han hoac bi thu quyen. Panel phai hien state rieng (AC-25).
    Auth(String),
    /// Khong toi duoc host: rot VPN, mat mang, DNS. Panel giu snapshot cu (AC-24).
    Network(String),
    Api { status: u16, body: String },
    Parse(String),
}

impl JiraError {
    pub fn kind(&self) -> &'static str {
        match self {
            JiraError::Auth(_) => "auth",
            JiraError::Network(_) => "network",
            JiraError::Api { .. } => "api",
            JiraError::Parse(_) => "parse",
        }
    }
}

impl fmt::Display for JiraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JiraError::Auth(m) => write!(f, "Token het han hoac khong du quyen ({m})"),
            JiraError::Network(m) => write!(f, "Khong ket noi duoc Jira ({m})"),
            JiraError::Api { status, body } => {
                let short: String = body.chars().take(160).collect();
                write!(f, "Jira tra ve HTTP {status}: {short}")
            }
            JiraError::Parse(m) => write!(f, "Khong doc duoc du lieu Jira ({m})"),
        }
    }
}

impl std::error::Error for JiraError {}

// ------------------------------------------------------------ wire structs

#[derive(Debug, Deserialize)]
struct SprintPage {
    values: Vec<RawSprint>,
}

#[derive(Debug, Deserialize)]
struct RawSprint {
    id: u64,
    name: String,
    #[serde(default)]
    state: String,
    #[serde(rename = "startDate")]
    start_date: Option<String>,
    #[serde(rename = "endDate")]
    end_date: Option<String>,
    #[serde(rename = "originBoardId")]
    origin_board_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct IssuePage {
    #[serde(default)]
    total: u32,
    #[serde(default)]
    issues: Vec<RawIssue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawIssue {
    pub key: String,
    pub fields: RawFields,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawFields {
    #[serde(default)]
    pub summary: String,
    pub status: RawStatus,
    pub assignee: Option<RawUser>,
    pub created: String,
    pub updated: String,
    pub issuetype: Option<RawNamed>,
    pub priority: Option<RawNamed>,

    /// Story point — thua, chi 5/9 ticket dang mo co gia tri (F2)
    #[serde(rename = "customfield_10107", default)]
    pub story_point: Option<f64>,
    /// App task score — thua hon nua, 3/9 (F2)
    #[serde(rename = "customfield_10704", default)]
    pub app_task_score: Option<f64>,

    /// Approvers / QCs. De nguyen `Value` vi Jira co the tra ve mang object
    /// user hoac mang chuoi tuy cau hinh field; `usernames_from` chiu duoc ca hai.
    #[serde(rename = "customfield_10200", default)]
    pub approvers: Option<serde_json::Value>,
    #[serde(rename = "customfield_10201", default)]
    pub qcs: Option<serde_json::Value>,
}

/// Rut dinh danh nguoi tu mot field kieu mang, chiu duoc ca `["ten"]`,
/// `[{"name": "ten", ...}]` (DC) lan `[{"accountId": "...", ...}]` (Cloud).
pub fn usernames_from(v: &Option<serde_json::Value>) -> Vec<String> {
    let Some(serde_json::Value::Array(arr)) = v else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|x| match x {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Object(o) => o
                .get("name")
                .or_else(|| o.get("accountId"))
                .and_then(|n| n.as_str())
                .map(String::from),
            _ => None,
        })
        .collect()
}

/// Mot board trong ket qua `--list-boards`.
#[derive(Debug, Clone, Deserialize)]
pub struct BoardInfo {
    pub id: u64,
    pub name: String,
    #[serde(rename = "type", default)]
    pub board_type: String,
}

#[derive(Debug, Deserialize)]
struct IssueTypeStatuses {
    #[serde(default)]
    statuses: Vec<NamedStatus>,
}

#[derive(Debug, Deserialize)]
struct NamedStatus {
    name: String,
}

#[derive(Debug, Deserialize)]
struct BoardPage {
    #[serde(default)]
    values: Vec<BoardInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawStatus {
    pub name: String,
    #[serde(rename = "statusCategory")]
    pub category: RawStatusCategory,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawStatusCategory {
    /// "new" | "indeterminate" | "done"
    pub key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawUser {
    /// Server/DC dung `name`; giu Option phong khi instance tra thieu.
    #[serde(default)]
    pub name: Option<String>,
    /// Jira Cloud (GDPR) KHONG co `name` — dinh danh la `accountId`.
    #[serde(rename = "accountId", default)]
    pub account_id: Option<String>,
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,
}

impl RawUser {
    /// Khoa dinh danh hieu dung: `name` (DC) truoc, `accountId` (Cloud) sau.
    /// Moi phep so khop nguoi (assignee, `me`, mau sac) deu phai di qua day —
    /// tren Cloud ma doc `name` truc tiep thi 100% ticket thanh "chua giao".
    pub fn username(&self) -> Option<String> {
        self.name
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| self.account_id.clone().filter(|s| !s.is_empty()))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawNamed {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SprintMeta {
    pub id: u64,
    pub name: String,
    pub board_id: u64,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

/// Hai URL cung goc (scheme + host + port)?
///
/// Dung lam hang rao chong confused-deputy o hai cho: (1) token DA LUU chi
/// duoc gan vao request toi dung host da luu trong config, (2) `open_issue`
/// chi mo link tro ve dung Jira instance. So sanh chuoi kieu `starts_with`
/// khong du: `https://jira.example.com.evil.com` va
/// `https://jira.example.com@evil.com` deu qua mat prefix check.
pub fn same_origin(a: &str, b: &str) -> bool {
    let (Ok(ua), Ok(ub)) = (reqwest::Url::parse(a), reqwest::Url::parse(b)) else {
        return false;
    };
    ua.scheme() == ub.scheme()
        && ua.host_str().is_some()
        && ua.host_str() == ub.host_str()
        && ua.port_or_known_default() == ub.port_or_known_default()
}

// ------------------------------------------------------------ date parsing

/// Jira Server tra ve `2026-07-22T14:30:35.000+0700` — offset KHONG co dau hai cham,
/// nen `parse_from_rfc3339` se fail. Thu ca hai dang.
pub fn parse_jira_datetime(s: &str) -> Result<DateTime<Utc>, JiraError> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    for fmt in ["%Y-%m-%dT%H:%M:%S%.f%z", "%Y-%m-%dT%H:%M:%S%z"] {
        if let Ok(dt) = DateTime::<FixedOffset>::parse_from_str(s, fmt) {
            return Ok(dt.with_timezone(&Utc));
        }
    }
    Err(JiraError::Parse(format!("timestamp la: {s}")))
}

// ------------------------------------------------------------------ client

/// Cach ky vao tung request. Ba duong khac nhau ve header LAN vong doi bi mat:
/// PAT/API-token song lau va bat bien; access token cua OAuth ngan han va do
/// `TokenStore` xoay — client phai hoi lai truoc moi request.
pub enum JiraAuth {
    /// Jira DC/Server PAT — `Authorization: Bearer`.
    Pat(String),
    /// Jira Cloud API token — `Authorization: Basic b64(email:token)`.
    Basic { email: String, token: String },
    /// Jira Cloud OAuth 3LO — Bearer access token lay tu TokenStore.
    Oauth(std::sync::Arc<crate::oauth::TokenStore>),
}

pub struct JiraClient {
    http: reqwest::Client,
    /// Base cho link mo browser (`/browse/KEY`) — luon la site URL.
    base: String,
    /// Base cho REST API. Voi OAuth la `api.atlassian.com/ex/jira/{cloud_id}`,
    /// hai mode con lai trung voi `base`.
    api: String,
    auth: JiraAuth,
    sprint_cache: Mutex<Option<(SprintMeta, Instant)>>,
}

impl JiraClient {
    pub fn new(base_url: &str, api_base: Option<String>, auth: JiraAuth) -> Result<Self> {
        // Cert cua jira.example.com do GlobalSign cap va con han
        // -> GIU verification nghiem ngat, khong dung danger_accept_invalid_certs.
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .user_agent("master-jira/0.1")
            .build()?;
        let base = base_url.trim_end_matches('/').to_string();
        let api = api_base
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| base.clone());
        Ok(Self {
            http,
            base,
            api,
            auth,
            sprint_cache: Mutex::new(None),
        })
    }

    /// Tien loi cho DC PAT — giu cho CLI va test khoi dai dong.
    pub fn new_pat(base_url: &str, token: String) -> Result<Self> {
        Self::new(base_url, None, JiraAuth::Pat(token))
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, JiraError> {
        // OAuth: access token co the vua het han giua hai lan poll. Gap 401 thi
        // ep refresh dung MOT lan roi goi lai; van 401 nua nghia la refresh token
        // cung chet -> tra Auth de panel hien state "token het han".
        let mut da_refresh = false;
        loop {
            let url = format!("{}{}", self.api, path);
            log::debug!("GET {url}");

            // Access token da dung cho luot nay — de refresh-neu-stale biet
            // "co ai refresh truoc minh chua" ma khong dot them luot xoay vong.
            let mut used_access: Option<String> = None;
            let mut req = self.http.get(&url).header("Accept", "application/json");
            req = match &self.auth {
                JiraAuth::Pat(t) => req.bearer_auth(t),
                JiraAuth::Basic { email, token } => req.basic_auth(email, Some(token)),
                JiraAuth::Oauth(store) => {
                    let tok = store.access_token().await.map_err(JiraError::Auth)?;
                    used_access = Some(tok.clone());
                    req.bearer_auth(tok)
                }
            };

            let resp = req.send().await.map_err(|e| {
                if e.is_timeout() {
                    JiraError::Network(format!("timeout sau {}s", HTTP_TIMEOUT.as_secs()))
                } else {
                    JiraError::Network(e.to_string())
                }
            })?;

            let status = resp.status();
            if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                if let JiraAuth::Oauth(store) = &self.auth {
                    if !da_refresh && status == reqwest::StatusCode::UNAUTHORIZED {
                        da_refresh = true;
                        if store
                            .force_refresh_if_stale(used_access.as_deref().unwrap_or(""))
                            .await
                            .is_ok()
                        {
                            continue;
                        }
                    }
                }
                return Err(JiraError::Auth(format!("HTTP {}", status.as_u16())));
            }
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(JiraError::Api {
                    status: status.as_u16(),
                    body,
                });
            }

            return resp
                .json::<T>()
                .await
                .map_err(|e| JiraError::Parse(e.to_string()));
        }
    }

    /// Ai dang cam token nay. Dung cho nut "Kiem tra ket noi": vua xac thuc PAT
    /// vua tra ve username de dien thang vao o `me` — nguoi dung khong phai tu
    /// di mo Jira ra tra cuu username cua chinh minh.
    pub async fn myself(&self) -> Result<RawUser, JiraError> {
        self.get_json("/rest/api/2/myself").await
    }

    /// Toan bo status co trong workflow cua project.
    ///
    /// Can cho o chon status trong Settings: de nguoi dung go tay thi go sai
    /// mot chu la loc ra 0 ticket ma panel im lang, khong bao gi.
    pub async fn project_statuses(&self, project: &str) -> Result<Vec<String>, JiraError> {
        let per_type: Vec<IssueTypeStatuses> = self
            .get_json(&format!("/rest/api/2/project/{project}/statuses"))
            .await?;
        let mut out: Vec<String> = per_type
            .into_iter()
            .flat_map(|t| t.statuses)
            .map(|s| s.name)
            .collect();
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// Liet ke board cua mot project — de anh biet dien `board_id` nao vao config.
    /// Khong ai nho duoc board id, va Jira khong hien no o giao dien.
    pub async fn list_boards(&self, project: &str) -> Result<Vec<BoardInfo>, JiraError> {
        let page: BoardPage = self
            .get_json(&format!(
                "/rest/agile/1.0/board?projectKeyOrId={project}&maxResults=100"
            ))
            .await?;
        Ok(page.values)
    }

    /// Sprint dang chay cua board. Cache 10 phut de steady-state chi con
    /// 1 request/phut (AC-3), nhung van tu bam theo khi sang sprint moi (AC-1).
    pub async fn active_sprint(&self, board_id: u64) -> Result<Option<SprintMeta>, JiraError> {
        {
            let cache = self.sprint_cache.lock().await;
            if let Some((meta, at)) = cache.as_ref() {
                if at.elapsed() < SPRINT_CACHE_TTL {
                    return Ok(Some(meta.clone()));
                }
            }
        }

        let page: SprintPage = self
            .get_json(&format!(
                "/rest/agile/1.0/board/{board_id}/sprint?state=active"
            ))
            .await?;

        // Board co the tra ve nhieu sprint active; lay cai co endDate gan nhat.
        let mut best: Option<SprintMeta> = None;
        for s in page.values {
            if !s.state.eq_ignore_ascii_case("active") && !s.state.is_empty() {
                continue;
            }
            let meta = SprintMeta {
                id: s.id,
                name: s.name,
                board_id: s.origin_board_id.unwrap_or(board_id),
                start: s.start_date.as_deref().and_then(|d| parse_jira_datetime(d).ok()),
                end: s.end_date.as_deref().and_then(|d| parse_jira_datetime(d).ok()),
            };
            best = match best {
                None => Some(meta),
                Some(cur) => {
                    let take = match (meta.end, cur.end) {
                        (Some(a), Some(b)) => a < b,
                        (Some(_), None) => true,
                        _ => false,
                    };
                    Some(if take { meta } else { cur })
                }
            };
        }

        if let Some(meta) = best.as_ref() {
            let mut cache = self.sprint_cache.lock().await;
            *cache = Some((meta.clone(), Instant::now()));
            log::info!("sprint dang chay: {} (id {})", meta.name, meta.id);
        } else {
            log::warn!("board {board_id} khong co sprint dang chay");
        }
        Ok(best)
    }

    /// Xoa cache sprint — dung khi anh bam "Refresh now" de ep resolve lai.
    pub async fn invalidate_sprint_cache(&self) {
        *self.sprint_cache.lock().await = None;
    }

    pub async fn sprint_issues(&self, sprint_id: u64) -> Result<Vec<RawIssue>, JiraError> {
        let mut out: Vec<RawIssue> = Vec::new();
        let mut start_at: u32 = 0;
        loop {
            let page: IssuePage = self
                .get_json(&format!(
                    "/rest/agile/1.0/sprint/{sprint_id}/issue\
                     ?fields={FIELDS}&startAt={start_at}&maxResults={PAGE_SIZE}"
                ))
                .await?;

            let got = page.issues.len() as u32;
            out.extend(page.issues);

            if got == 0 || out.len() as u32 >= page.total || got < PAGE_SIZE {
                break;
            }
            start_at += got;
            if start_at > 2000 {
                log::warn!("dung phan trang o {start_at} — sprint bat thuong lon");
                break;
            }
        }
        log::info!("sprint {sprint_id}: lay duoc {} issue", out.len());
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duoc_dinh_dang_offset_khong_hai_cham_cua_jira_server() {
        // Dang that Jira Server tra ve — day chinh la cho parse_from_rfc3339 chet
        let dt = parse_jira_datetime("2026-07-22T14:30:35.000+0700").unwrap();
        assert_eq!(dt.to_rfc3339(), "2026-07-22T07:30:35+00:00");
    }

    #[test]
    fn van_parse_duoc_rfc3339_chuan() {
        let dt = parse_jira_datetime("2026-07-06T14:50:00.000+07:00").unwrap();
        assert_eq!(dt.to_rfc3339(), "2026-07-06T07:50:00+00:00");
    }

    #[test]
    fn parse_khong_co_mili_giay() {
        assert!(parse_jira_datetime("2026-07-22T14:30:35+0700").is_ok());
    }

    #[test]
    fn timestamp_rac_thi_bao_loi_chu_khong_panic() {
        assert!(parse_jira_datetime("hom qua").is_err());
    }

    #[test]
    fn deserialize_duoc_issue_that() {
        // Payload rut gon tu PROJ-1910 (raw REST tra so tran, khong boc {"value":..})
        let raw = r#"{
          "key": "PROJ-1910",
          "fields": {
            "summary": "ticket test",
            "status": { "name": "Pending",
                        "statusCategory": { "key": "new", "name": "To Do" } },
            "assignee": { "name": "gale.shaw",
                          "displayName": "Gale Shaw - Engineering" },
            "created": "2026-05-29T16:00:20.000+0700",
            "updated": "2026-07-07T17:53:22.000+0700",
            "issuetype": { "name": "Task" },
            "priority": { "name": "Medium" },
            "customfield_10107": 3.0,
            "customfield_10704": 3.0
          }
        }"#;
        let issue: RawIssue = serde_json::from_str(raw).unwrap();
        assert_eq!(issue.key, "PROJ-1910");
        assert_eq!(issue.fields.status.category.key, "new");
        assert_eq!(issue.fields.story_point, Some(3.0));
        assert_eq!(issue.fields.assignee.unwrap().name.unwrap(), "gale.shaw");
    }

    #[test]
    fn same_origin_chan_cac_kieu_gia_mao_host() {
        assert!(same_origin(
            "https://jira.example.com:8443/browse/PROJ-1",
            "https://jira.example.com:8443"
        ));
        assert!(same_origin("https://a.com/x", "https://a.com:443/y"), "port mac dinh");
        // Cac kieu qua mat duoc prefix check nhung KHONG duoc qua day:
        assert!(!same_origin("https://jira.example.com.evil.com/x", "https://jira.example.com"));
        assert!(!same_origin("https://jira.example.com@evil.com/x", "https://jira.example.com"));
        assert!(!same_origin("http://jira.example.com/x", "https://jira.example.com"));
        assert!(!same_origin("https://jira.example.com:9000/x", "https://jira.example.com:8443"));
        assert!(!same_origin("khong-phai-url", "https://jira.example.com"));
        assert!(!same_origin("https://evil.com", ""));
    }

    #[test]
    fn user_cloud_khong_co_name_thi_dinh_danh_bang_account_id() {
        // Jira Cloud (GDPR) bo han field `name` — assignee chi con accountId.
        let raw = r#"{ "accountId": "712020:aa-bb-cc", "displayName": "Gale Shaw" }"#;
        let u: RawUser = serde_json::from_str(raw).unwrap();
        assert_eq!(u.name, None);
        assert_eq!(u.username().unwrap(), "712020:aa-bb-cc");

        // DC co ca hai thi `name` phai thang.
        let raw_dc = r#"{ "name": "gale.shaw", "accountId": "x", "displayName": "Gale" }"#;
        let dc: RawUser = serde_json::from_str(raw_dc).unwrap();
        assert_eq!(dc.username().unwrap(), "gale.shaw");
    }

    #[test]
    fn usernames_from_doc_duoc_ca_name_lan_account_id() {
        let v = Some(serde_json::json!([
            { "name": "sam.hale" },
            { "accountId": "5b10ac8d82e05b22cc7d4ef5" },
            "chuoi.tran"
        ]));
        assert_eq!(
            usernames_from(&v),
            vec!["sam.hale", "5b10ac8d82e05b22cc7d4ef5", "chuoi.tran"]
        );
    }

    #[test]
    fn issue_khong_assignee_va_khong_story_point_van_deserialize_duoc() {
        let raw = r#"{
          "key": "PROJ-2070",
          "fields": {
            "summary": "bug",
            "status": { "name": "In Progress",
                        "statusCategory": { "key": "indeterminate", "name": "In Progress" } },
            "assignee": null,
            "created": "2026-07-10T10:14:00.000+0700",
            "updated": "2026-07-22T10:41:21.000+0700"
          }
        }"#;
        let issue: RawIssue = serde_json::from_str(raw).unwrap();
        assert!(issue.fields.assignee.is_none());
        assert!(issue.fields.story_point.is_none());
        assert!(issue.fields.issuetype.is_none());
    }
}
