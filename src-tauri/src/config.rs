//! Config + token resolution.
//!
//! Token KHONG BAO GIO nam trong repo. Thu tu uu tien khi lay token:
//!   1. bien moi truong `JIRA_WIDGET_TOKEN` (tien cho dev/test)
//!   2. macOS Keychain, service `jira-widget` / account `jira-pat`  <-- mac dinh
//!   3. field `token` trong config.toml (fallback, file bi ep chmod 600)
//!
//! Dung `security` CLI cua macOS thay vi crate `keyring`: bot mot dependency,
//! bot rui ro doi API giua cac major version, va app nay von chi chay tren macOS.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

pub const KEYCHAIN_SERVICE: &str = "jira-widget";
pub const KEYCHAIN_ACCOUNT: &str = "jira-pat";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotifyConfig {
    /// Bao khi ticket doi status
    pub status_changed: bool,
    /// Bao khi ticket doi assignee
    pub assignee_changed: bool,
    /// Bao khi co ticket moi vao sprint
    pub added: bool,
    /// Bao khi ticket bi go khoi sprint
    pub removed: bool,
    /// Vuot nguong nay thi gom thanh 1 notification tong hop (AC-23)
    pub group_threshold: usize,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self {
            status_changed: true,
            assignee_changed: true,
            added: true,
            removed: true,
            group_threshold: 3,
        }
    }
}

/// Panel nam o tang nao tren man hinh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowLayer {
    /// Dan vao nen desktop: nam duoi MOI cua so app, khong bao gio che thu gi.
    /// Nhin thay khi desktop lo ra — kieu widget cua Ubersicht.
    Desktop,
    /// Noi tren tat ca, luon nhin thay ke ca khi dang lam viec o app khac.
    Floating,
}

impl Default for WindowLayer {
    fn default() -> Self {
        WindowLayer::Desktop
    }
}

/// Panel dang hien viec cua ai.
///
/// `Team` la goc nhin leader — ca sprint, moi nguoi. `OnlyMe` de mot member
/// chuyen qua xem rieng viec cua minh tren cung cai widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayMode {
    Team,
    OnlyMe,
}

impl Default for DisplayMode {
    fn default() -> Self {
        DisplayMode::Team
    }
}

impl DisplayMode {
    /// Only Me ma khong biet `me` la ai thi loc ra rong tuyet doi — panel trang
    /// khong loi giai thich. Ve `Team` va noi ra trong log (AC-D5).
    pub fn effective_for(self, me: &str) -> DisplayMode {
        match self {
            DisplayMode::OnlyMe if me.trim().is_empty() => {
                log::warn!("display_mode = only_me nhung chua dat `me` — quay ve team");
                DisplayMode::Team
            }
            m => m,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DisplayMode::Team => "team",
            DisplayMode::OnlyMe => "only_me",
        }
    }

    /// Doc tu chuoi UI/tray gui xuong. Gia tri la thi ve `Team` chu khong panic.
    pub fn from_str_or_team(s: &str) -> DisplayMode {
        match s {
            "only_me" => DisplayMode::OnlyMe,
            _ => DisplayMode::Team,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub jira_url: String,
    /// Key cua project, vd "PROJ". Dung de loc issue, dat nhan, va cho
    /// `--list-boards`. Mot board co the chua issue cua nhieu project nen
    /// khong bo qua duoc buoc loc nay.
    pub project_key: String,
    /// Board id. Chua biet so may thi chay `jira-widget --list-boards`.
    /// Scoped theo board de khong dinh sprint cua board khac (AC-2).
    pub board_id: u64,
    /// Username Jira cua chinh anh, vd "sam.hale". De trong thi tat cac tinh
    /// nang lien quan toi ca nhan (to dam dong cua minh, hang doi cho duyet).
    pub me: String,
    pub poll_interval_secs: u64,
    /// Ticket khong duoc dung toi qua ngan nay ngay -> canh bao stale (AC-13)
    pub stale_days: i64,
    /// Con it hon ngan nay gio la "sap het sprint" (AC-13)
    pub ending_soon_hours: i64,
    /// Ticket song lau hon ngan nay thi to canh bao o cot age (AC-20)
    pub old_age_days: i64,
    // --- Ba hang doi. Moi cai mot vai tro khac nhau trong workflow, va loc
    // theo MOT field nguoi khac nhau. Da kiem chung tren project:
    //   Approvers (cf_10200) = leader duyet   -> vd alex.lee
    //   QCs       (cf_10201) = nguoi di test  -> vd blake.kim
    // Ban truoc gop hai field lam mot (union) — sai, vi no tron lan hai vai tro.
    /// Cho TEST — loc theo field QCs
    pub test_statuses: Vec<String>,
    pub show_test_queue: bool,
    /// Cho DUYET — loc theo field Approvers
    pub review_statuses: Vec<String>,
    pub show_review_queue: bool,
    /// Cho RELEASE — khong loc theo nguoi, day la viec cua ca team.
    /// Dong thoi dung de tach "da xong" khoi "moi cho release" o thanh tien do.
    pub pending_release_statuses: Vec<String>,
    pub show_release_queue: bool,
    pub notify: NotifyConfig,
    /// `desktop` (mac dinh) hay `floating`. Doi xong khoi dong lai app.
    pub window_layer: WindowLayer,
    /// `team` (mac dinh) hay `only_me`. Doi duoc NONG, khong can khoi dong lai.
    pub display_mode: DisplayMode,
    /// Fallback khi khong dung Keychain. De trong o cau hinh mac dinh.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            jira_url: "https://jira.example.com".into(),
            project_key: "PROJ".into(),
            board_id: 1000,
            me: String::new(),
            poll_interval_secs: 60,
            stale_days: 3,
            ending_soon_hours: 24,
            old_age_days: 30,
            // `Ready for Review` la status duyet that su cua project. Ban dau dat
            // ["Ready for Test", "In Test", "Ready for Release"] vi tuong project
            // khong co status Review — ket luan voi tu danh sach status THAY TRONG
            // SPRINT chu khong phai workflow, va no sai.
            test_statuses: vec!["Ready for Test".into(), "In Test".into()],
            show_test_queue: true,
            review_statuses: vec!["Ready for Review".into()],
            show_review_queue: true,
            pending_release_statuses: vec!["Ready for Release".into()],
            show_release_queue: true,
            notify: NotifyConfig::default(),
            window_layer: WindowLayer::default(),
            display_mode: DisplayMode::default(),
            token: None,
        }
    }
}

impl Config {
    /// Mode thuc su duoc ap dung, da tinh ca truong hop `me` rong (AC-D5).
    pub fn effective_display_mode(&self) -> DisplayMode {
        self.display_mode.effective_for(&self.me)
    }
}

pub fn config_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("khong doc duoc $HOME")?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join(KEYCHAIN_SERVICE))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// Doc config; neu chua co thi ghi ban mac dinh ra dia de anh sua tay duoc.
pub fn load() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        let cfg = Config::default();
        save(&cfg)?;
        log::info!("da tao config mac dinh tai {}", path.display());
        return Ok(cfg);
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("khong doc duoc {}", path.display()))?;
    let cfg: Config = toml::from_str(&raw)
        .with_context(|| format!("config.toml sai cu phap: {}", path.display()))?;
    Ok(cfg)
}

/// Ghi config theo kieu NGUYEN TU: ra file tam roi rename de len.
///
/// `fs::write` cat file truoc khi ghi, nen chet giua chung la con lai mot
/// config.toml RONG — mat luon field `token` fallback neu dang dung che do do.
/// Truoc day `save()` chi goi tu mot cu bam Luu; gio no con nam tren duong doi
/// display mode, tuc la co the goi lien tuc tu webview, nen cua so do rong ra
/// dang ke. `rename` trong cung thu muc la nguyen tu: doc gia thay ban cu hoac
/// ban moi, khong bao gio thay ban rong.
///
/// Quyen 600 dat luc MO file, truoc khi token kip cham dia — dat sau se ho
/// mot khe file 644 co noi dung nhay cam.
pub fn save(cfg: &Config) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let dir = config_dir()?;
    fs::create_dir_all(&dir)?;
    let path = config_path()?;
    let body = toml::to_string_pretty(cfg)?;

    let tmp = path.with_extension("toml.tmp");
    {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600) // (AC-6)
            .open(&tmp)
            .with_context(|| format!("khong mo duoc {}", tmp.display()))?;
        f.write_all(body.as_bytes())?;
        f.sync_all()?; // ep xuong dia truoc khi rename, khong thi rename ve file rong
    }
    fs::rename(&tmp, &path)
        .with_context(|| format!("khong thay duoc {}", path.display()))?;
    Ok(())
}

// ---------------------------------------------------------------- Keychain

pub fn keychain_get() -> Option<String> {
    let out = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            KEYCHAIN_ACCOUNT,
            "-w",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let token = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

pub fn keychain_set(token: &str) -> Result<()> {
    let status = Command::new("security")
        .args([
            "add-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            KEYCHAIN_ACCOUNT,
            "-w",
            token,
            "-U",
        ])
        .status()
        .context("khong chay duoc `security` CLI")?;
    if !status.success() {
        return Err(anyhow!("security add-generic-password that bai"));
    }
    Ok(())
}

/// Lay token theo thu tu uu tien. Loi o day la loi "khong the chay tiep".
pub fn resolve_token(cfg: &Config) -> Result<String> {
    if let Ok(t) = std::env::var("JIRA_WIDGET_TOKEN") {
        let t = t.trim().to_string();
        if !t.is_empty() {
            log::info!("token: lay tu bien moi truong JIRA_WIDGET_TOKEN");
            return Ok(t);
        }
    }
    if let Some(t) = keychain_get() {
        log::info!("token: lay tu macOS Keychain");
        return Ok(t);
    }
    if let Some(t) = cfg.token.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        log::warn!("token: lay tu config.toml — nen chuyen sang Keychain bang `--setup-token`");
        return Ok(t.to_string());
    }
    Err(anyhow!(
        "chua co Jira token. Cap PAT cua rieng anh roi chay:\n  \
         jira-widget --set-token"
    ))
}

/// Nap PAT cua chinh nguoi dung vao Keychain.
///
/// Doc tu STDIN chu khong nhan qua tham so dong lenh: tham so nam trong `ps`
/// va trong lich su shell, tuc la token se lo ra cho moi tien trinh khac tren may.
pub fn set_token_from_stdin() -> Result<()> {
    use std::io::Read;

    eprintln!("Dan Jira Personal Access Token roi bam Enter (khong hien ra man hinh):");
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("khong doc duoc token tu stdin")?;
    let token = buf.trim();

    if token.is_empty() {
        return Err(anyhow!("token rong — khong luu gi ca"));
    }
    // PAT cua Jira DC dai 44 ky tu; canh bao chu khong chan, phong khi Atlassian doi.
    if token.len() < 20 {
        eprintln!("Canh bao: token chi dai {} ky tu, trong khong giong PAT.", token.len());
    }

    keychain_set(token)?;
    println!("OK — da luu PAT vao Keychain ({KEYCHAIN_SERVICE}/{KEYCHAIN_ACCOUNT}).");
    println!("App khong con dung token bot nua.");
    Ok(())
}

/// Xoa token khoi Keychain — dung khi muon go han token bot cu ra khoi may.
pub fn clear_token() -> Result<()> {
    let out = Command::new("security")
        .args([
            "delete-generic-password",
            "-s",
            KEYCHAIN_SERVICE,
            "-a",
            KEYCHAIN_ACCOUNT,
        ])
        .output()
        .context("khong chay duoc `security` CLI")?;
    if out.status.success() {
        println!("Da xoa token khoi Keychain.");
    } else {
        println!("Khong co token nao trong Keychain de xoa.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_khop_voi_du_lieu_da_verify() {
        let c = Config::default();
        assert_eq!(c.board_id, 1000);
        assert_eq!(c.poll_interval_secs, 60);
        assert_eq!(c.stale_days, 3);
        // project khong co status "Review" — F3
        assert_eq!(c.review_statuses, vec!["Ready for Review"]);
        assert_eq!(c.test_statuses, vec!["Ready for Test", "In Test"]);
        assert!(c.show_test_queue && c.show_review_queue && c.show_release_queue);
        assert!(c.pending_release_statuses.contains(&"Ready for Release".to_string()));
        assert!(c.token.is_none(), "config mac dinh khong duoc chua token");
        assert_eq!(
            c.window_layer,
            WindowLayer::Desktop,
            "mac dinh phai dan vao desktop, khong duoc che app khac"
        );
    }

    #[test]
    fn config_cu_thieu_window_layer_van_doc_duoc() {
        // File config sinh truoc khi them tuy chon nay khong co dong window_layer.
        // `#[serde(default)]` o muc struct phai lo duoc, khong duoc bao loi.
        let cu = r#"
            jira_url = "https://jira.example.com"
            board_id = 1000
            poll_interval_secs = 60
        "#;
        let c: Config = toml::from_str(cu).unwrap();
        assert_eq!(c.window_layer, WindowLayer::Desktop);
        assert_eq!(c.stale_days, 3, "cac field thieu khac cung phai ve mac dinh");
    }

    #[test]
    fn window_layer_doc_duoc_ca_hai_gia_tri() {
        let d: Config = toml::from_str("window_layer = \"desktop\"").unwrap();
        assert_eq!(d.window_layer, WindowLayer::Desktop);
        let f: Config = toml::from_str("window_layer = \"floating\"").unwrap();
        assert_eq!(f.window_layer, WindowLayer::Floating);
    }

    #[test]
    fn display_mode_mac_dinh_la_team_va_config_cu_van_doc_duoc() {
        // AC-D1: file config sinh truoc tinh nang nay khong co dong display_mode.
        assert_eq!(Config::default().display_mode, DisplayMode::Team);
        let cu: Config = toml::from_str(r#"jira_url = "https://jira.example.com""#).unwrap();
        assert_eq!(cu.display_mode, DisplayMode::Team);
    }

    #[test]
    fn display_mode_doc_duoc_ca_hai_gia_tri() {
        let t: Config = toml::from_str("display_mode = \"team\"").unwrap();
        assert_eq!(t.display_mode, DisplayMode::Team);
        let o: Config = toml::from_str("display_mode = \"only_me\"").unwrap();
        assert_eq!(o.display_mode, DisplayMode::OnlyMe);
    }

    #[test]
    fn only_me_khong_co_me_thi_ve_team() {
        // AC-D5: sua tay config thanh only_me trong khi `me` rong -> khong duoc
        // de panel rong khong loi giai thich, phai tu ve team.
        let mut c = Config::default();
        c.display_mode = DisplayMode::OnlyMe;
        c.me = String::new();
        assert_eq!(c.effective_display_mode(), DisplayMode::Team);

        c.me = "   ".into(); // khoang trang cung tinh la rong
        assert_eq!(c.effective_display_mode(), DisplayMode::Team);

        c.me = "alex.lee".into();
        assert_eq!(c.effective_display_mode(), DisplayMode::OnlyMe);
    }

    #[test]
    fn display_mode_qua_lai_voi_chuoi() {
        assert_eq!(DisplayMode::from_str_or_team("only_me"), DisplayMode::OnlyMe);
        assert_eq!(DisplayMode::from_str_or_team("team"), DisplayMode::Team);
        assert_eq!(
            DisplayMode::from_str_or_team("gi do la"),
            DisplayMode::Team,
            "gia tri la thi ve mac dinh chu khong panic"
        );
        assert_eq!(DisplayMode::OnlyMe.as_str(), "only_me");
    }

    #[test]
    fn config_roundtrip_toml() {
        let c = Config::default();
        let s = toml::to_string_pretty(&c).unwrap();
        assert!(!s.contains("token ="), "token khong duoc ghi ra khi None");
        let back: Config = toml::from_str(&s).unwrap();
        assert_eq!(back.board_id, c.board_id);
        assert_eq!(back.review_statuses, c.review_statuses);
    }
}
