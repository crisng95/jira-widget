//! Cua so cai dat.
//!
//! Vi sao la CUA SO RIENG chu khong nhet vao panel: panel chay o tang desktop
//! (`window_layer = "desktop"`), tang do rat kho nhan ban phim — go text vao se
//! hong. Cua so nay la cua so binh thuong, co vien, nhan focus duoc.
//!
//! Danh doi ve bao mat, noi thang: truoc day PAT khong bao gio di qua webview
//! (chi co CLI `--set-token` doc tu stdin). Co o nhap token tren UI thi token
//! CO di qua webview mot lan. Giam thieu:
//!   - o nhap la `type=password`, xoa khoi state JS ngay sau khi gui
//!   - khong bao gio doc nguoc token ra UI — UI chi biet "co token" hay "chua"
//!   - CSP chan moi ket noi ra ngoai tu webview nen khong the tuon di dau
//!   - luu xong la vao Keychain, khong ghi vao config.toml

use crate::config::{self, Config, DisplayMode, NotifyConfig, WindowLayer};
use crate::jira::JiraClient;
use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub const SETTINGS_WINDOW: &str = "settings";

/// Ban camelCase cua `NotifyConfig` danh RIENG cho IPC.
///
/// Vi sao phai tach: `NotifyConfig` phuc vu hai chu — file TOML muon
/// `status_changed`, JSON cho webview muon `statusChanged`. Truoc day
/// `SettingsDto` nhung thang `NotifyConfig` vao, ma `rename_all` cua struct cha
/// KHONG lan xuong struct con, nen Rust gui `status_changed` con UI doc
/// `statusChanged` -> `undefined` -> checkbox luon tat. `added`/`removed` song
/// sot vi mot chu, khong doi dang. Them `rename_all` thang vao `NotifyConfig`
/// se lam hong config.toml dang co, nen tach DTO rieng.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyDto {
    pub status_changed: bool,
    pub assignee_changed: bool,
    pub added: bool,
    pub removed: bool,
    pub group_threshold: usize,
}

impl From<&NotifyConfig> for NotifyDto {
    fn from(c: &NotifyConfig) -> Self {
        Self {
            status_changed: c.status_changed,
            assignee_changed: c.assignee_changed,
            added: c.added,
            removed: c.removed,
            group_threshold: c.group_threshold,
        }
    }
}

impl From<NotifyDto> for NotifyConfig {
    fn from(d: NotifyDto) -> Self {
        Self {
            status_changed: d.status_changed,
            assignee_changed: d.assignee_changed,
            added: d.added,
            removed: d.removed,
            group_threshold: d.group_threshold,
        }
    }
}

/// Ban config gui cho UI. KHONG bao gio chua token — chi co co `has_token`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub jira_url: String,
    pub project_key: String,
    pub board_id: u64,
    pub me: String,
    pub poll_interval_secs: u64,
    pub stale_days: i64,
    pub ending_soon_hours: i64,
    pub old_age_days: i64,
    pub test_statuses: Vec<String>,
    pub show_test_queue: bool,
    pub review_statuses: Vec<String>,
    pub show_review_queue: bool,
    pub pending_release_statuses: Vec<String>,
    pub show_release_queue: bool,
    pub window_layer: String,
    /// "team" | "only_me"
    pub display_mode: String,
    pub notify: NotifyDto,
    /// Chi de UI hien "da co token" — khong phai gia tri token
    pub has_token: bool,
}

impl SettingsDto {
    pub fn from_config(c: &Config, has_token: bool) -> Self {
        Self {
            jira_url: c.jira_url.clone(),
            project_key: c.project_key.clone(),
            board_id: c.board_id,
            me: c.me.clone(),
            poll_interval_secs: c.poll_interval_secs,
            stale_days: c.stale_days,
            ending_soon_hours: c.ending_soon_hours,
            old_age_days: c.old_age_days,
            test_statuses: c.test_statuses.clone(),
            show_test_queue: c.show_test_queue,
            review_statuses: c.review_statuses.clone(),
            show_review_queue: c.show_review_queue,
            pending_release_statuses: c.pending_release_statuses.clone(),
            show_release_queue: c.show_release_queue,
            window_layer: match c.window_layer {
                WindowLayer::Desktop => "desktop".into(),
                WindowLayer::Floating => "floating".into(),
            },
            // Gui ban DA qua `effective_`: `me` rong thi UI phai thay "Ca team"
            // dung nhu cai panel dang chay, khong phai gia tri chet trong file.
            display_mode: c.effective_display_mode().as_str().into(),
            notify: NotifyDto::from(&c.notify),
            has_token,
        }
    }

    /// Gop vao config dang co, GIU NGUYEN field `token` fallback — neu khong
    /// thi moi lan bam Luu se xoa mat token cua nguoi dang dung che do do.
    fn merge_into(self, mut base: Config) -> Result<Config, String> {
        let url = self.jira_url.trim().trim_end_matches('/').to_string();
        if url.is_empty() {
            return Err("Jira URL khong duoc de trong".into());
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("Jira URL phai bat dau bang http:// hoac https://".into());
        }
        if self.board_id == 0 {
            return Err("Chua chon board".into());
        }
        if self.poll_interval_secs < 10 {
            return Err("Chu ky poll toi thieu 10 giay — thap hon la ep Jira vo ich".into());
        }
        if self.stale_days < 0 || self.old_age_days < 0 || self.ending_soon_hours < 0 {
            return Err("Nguong khong duoc am".into());
        }

        base.jira_url = url;
        base.project_key = self.project_key.trim().to_uppercase();
        base.board_id = self.board_id;
        base.me = self.me.trim().to_string();
        base.poll_interval_secs = self.poll_interval_secs;
        base.stale_days = self.stale_days;
        base.ending_soon_hours = self.ending_soon_hours;
        base.old_age_days = self.old_age_days;
        base.test_statuses = clean_list(self.test_statuses);
        base.show_test_queue = self.show_test_queue;
        base.review_statuses = clean_list(self.review_statuses);
        base.show_review_queue = self.show_review_queue;
        base.pending_release_statuses = clean_list(self.pending_release_statuses);
        base.show_release_queue = self.show_release_queue;
        base.window_layer = if self.window_layer == "floating" {
            WindowLayer::Floating
        } else {
            WindowLayer::Desktop
        };
        // `display_mode` CO TRONG dto nhung KHONG duoc ghi o day. No la thu duy
        // nhat doi nong duoc, nen radio goi thang `set_display_mode` — cai do
        // ghi config, dung lai snapshot va dong bo tick tray trong mot nhip.
        // Neu ghi ca o day thi: (1) bam "Chi luu" se ghi file mot dang con panel
        // chay mot neo, va (2) mode vua doi tu tray se bi form cu ghi de nguoc.
        base.notify = self.notify.into();
        Ok(base)
    }
}

fn clean_list(v: Vec<String>) -> Vec<String> {
    // `Vec::dedup()` chi bo trung LIEN TIEP — "In Test / Closed / In Test" se lot.
    // Giu nguyen thu tu nguoi dung go, chi bo cai lap lai.
    let mut seen = std::collections::HashSet::new();
    v.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .filter(|s| seen.insert(s.clone()))
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardDto {
    pub id: u64,
    pub name: String,
    pub board_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhoAmI {
    pub name: String,
    pub display_name: String,
}

pub fn open_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window(SETTINGS_WINDOW) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    match WebviewWindowBuilder::new(
        app,
        SETTINGS_WINDOW,
        WebviewUrl::App("settings.html".into()),
    )
    .title("Jira Widget — Cai dat")
    .inner_size(560.0, 600.0)
    .min_inner_size(500.0, 440.0)
    .resizable(true)
    .focused(true)
    // Cua so BINH THUONG: co vien, nhan ban phim. Nguoc han voi panel.
    .decorations(true)
    .always_on_top(false)
    .build()
    {
        Ok(_) => log::info!("mo cua so cai dat"),
        Err(e) => log::error!("khong mo duoc cua so cai dat: {e}"),
    }
}

// ---------------------------------------------------------------- commands

#[tauri::command]
pub fn settings_get() -> Result<SettingsDto, String> {
    let cfg = config::load().map_err(|e| e.to_string())?;
    let has = config::keychain_get().is_some()
        || cfg.token.as_deref().map(|t| !t.trim().is_empty()).unwrap_or(false)
        || std::env::var("JIRA_WIDGET_TOKEN").map(|t| !t.trim().is_empty()).unwrap_or(false);
    Ok(SettingsDto::from_config(&cfg, has))
}

#[tauri::command]
pub fn settings_save(dto: SettingsDto) -> Result<(), String> {
    let base = config::load().map_err(|e| e.to_string())?;
    let merged = dto.merge_into(base)?;
    config::save(&merged).map_err(|e| e.to_string())?;
    log::info!(
        "da luu cai dat: project={} board={} me={:?} layer={:?}",
        merged.project_key,
        merged.board_id,
        merged.me,
        merged.window_layer
    );
    Ok(())
}

/// Nhan token tu UI va cat thang vao Keychain. Khong ghi ra config.toml,
/// khong log gia tri, khong tra nguoc ve UI.
#[tauri::command]
pub fn settings_save_token(token: String) -> Result<(), String> {
    let t = token.trim();
    if t.is_empty() {
        return Err("Token rong".into());
    }
    config::keychain_set(t).map_err(|e| e.to_string())?;
    log::info!("da cap nhat PAT trong Keychain ({} ky tu)", t.len());
    Ok(())
}

#[tauri::command]
pub fn settings_clear_token() -> Result<(), String> {
    config::clear_token().map_err(|e| e.to_string())?;
    log::info!("da xoa PAT khoi Keychain");
    Ok(())
}

/// Kiem tra ket noi bang chinh cai URL + token dang co.
///
/// `token_override` cho phep thu token vua go MA CHUA luu — nguoi dung biet
/// token dung hay sai truoc khi ghi de cai cu.
#[tauri::command]
pub async fn settings_test_connection(
    jira_url: String,
    token_override: Option<String>,
) -> Result<WhoAmI, String> {
    let token = match token_override.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) {
        Some(t) => t,
        None => {
            let cfg = config::load().map_err(|e| e.to_string())?;
            config::resolve_token(&cfg).map_err(|e| e.to_string())?
        }
    };
    let client =
        JiraClient::new(jira_url.trim().trim_end_matches('/'), token).map_err(|e| e.to_string())?;
    let me = client.myself().await.map_err(|e| e.to_string())?;
    Ok(WhoAmI {
        name: me.name.clone().unwrap_or_default(),
        display_name: me.display_name.unwrap_or_else(|| me.name.unwrap_or_default()),
    })
}

/// Status that su co trong workflow cua project — cho o chon trong Settings.
///
/// Go tay la nguon goc cua ca mot lop loi im lang: `review_statuses` tung bi dat
/// thanh mot ten khong ton tai, panel loc ra 0 ticket va khong bao gi ca.
#[tauri::command]
pub async fn settings_project_statuses(
    jira_url: String,
    project_key: String,
    token_override: Option<String>,
) -> Result<Vec<String>, String> {
    let key = project_key.trim().to_uppercase();
    if key.is_empty() {
        return Err("Nhap project key truoc (vd PROJ)".into());
    }
    let token = match token_override.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) {
        Some(t) => t,
        None => {
            let cfg = config::load().map_err(|e| e.to_string())?;
            config::resolve_token(&cfg).map_err(|e| e.to_string())?
        }
    };
    let client =
        JiraClient::new(jira_url.trim().trim_end_matches('/'), token).map_err(|e| e.to_string())?;
    client.project_statuses(&key).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settings_list_boards(
    jira_url: String,
    project_key: String,
    token_override: Option<String>,
) -> Result<Vec<BoardDto>, String> {
    let key = project_key.trim().to_uppercase();
    if key.is_empty() {
        return Err("Nhap project key truoc (vd PROJ)".into());
    }
    // Cho dung token vua go ma chua bam Luu — neu khong thi lan dau cai dat
    // se ket: chua luu token thi khong tim duoc board.
    let token = match token_override.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) {
        Some(t) => t,
        None => {
            let cfg = config::load().map_err(|e| e.to_string())?;
            config::resolve_token(&cfg).map_err(|e| e.to_string())?
        }
    };
    let client =
        JiraClient::new(jira_url.trim().trim_end_matches('/'), token).map_err(|e| e.to_string())?;
    let boards = client.list_boards(&key).await.map_err(|e| e.to_string())?;
    Ok(boards
        .into_iter()
        .map(|b| BoardDto {
            id: b.id,
            name: b.name,
            board_type: b.board_type,
        })
        .collect())
}

/// Khoi dong lai app de cai dat moi co hieu luc toan bo.
///
/// Ap dung nong tung phan (poll interval, nguong) thi de, nhung `window_layer`
/// va viec doi token bat buoc phai dung lai client — khoi dong lai la cach
/// dung chac chan, khong co truong hop nua-cu-nua-moi.
#[tauri::command]
pub fn settings_apply_restart(app: tauri::AppHandle) {
    log::info!("khoi dong lai de ap dung cai dat");
    app.restart();
}

#[tauri::command]
pub fn settings_close(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window(SETTINGS_WINDOW) {
        let _ = w.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dto() -> SettingsDto {
        SettingsDto::from_config(&Config::default(), false)
    }

    /// Duyet moi key trong cay JSON, tra ve key nao con dang snake_case.
    fn keys_snake_case(v: &serde_json::Value, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Object(o) => {
                for (k, val) in o {
                    if k.contains('_') {
                        out.push(k.clone());
                    }
                    keys_snake_case(val, out);
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| keys_snake_case(x, out)),
            _ => {}
        }
    }

    #[test]
    fn moi_key_gui_cho_webview_deu_la_camel_case() {
        // Loi that da xay ra: `rename_all` cua struct cha KHONG lan xuong struct
        // con, nen `notify.status_changed` di sang UI o dang snake_case va
        // checkbox khong bao gio tich duoc. Test nay bat CA LOP loi do, khong
        // chi rieng cho vua sua.
        let json = serde_json::to_value(SettingsDto::from_config(&Config::default(), true)).unwrap();
        let mut bad = Vec::new();
        keys_snake_case(&json, &mut bad);
        assert!(bad.is_empty(), "key con dang snake_case: {bad:?}");
    }

    #[test]
    fn notify_di_ve_khong_mat_gia_tri() {
        let mut c = Config::default();
        c.notify.status_changed = false;
        c.notify.assignee_changed = false;
        c.notify.group_threshold = 9;

        let d = SettingsDto::from_config(&c, false);
        // qua JSON that su, dung cach webview nhan
        let wire: SettingsDto =
            serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
        let back = wire.merge_into(Config::default()).unwrap();

        assert!(!back.notify.status_changed);
        assert!(!back.notify.assignee_changed);
        assert!(back.notify.added, "cai khong doi phai giu nguyen");
        assert_eq!(back.notify.group_threshold, 9);
    }

    #[test]
    fn dto_khong_bao_gio_mang_token() {
        let mut c = Config::default();
        c.token = Some("bi-mat".into());
        let d = SettingsDto::from_config(&c, true);
        let json = serde_json::to_string(&d).unwrap();
        assert!(!json.contains("bi-mat"), "token khong duoc lot xuong UI");
        assert!(d.has_token);
    }

    #[test]
    fn luu_cai_dat_khong_lam_mat_token_fallback() {
        let mut base = Config::default();
        base.token = Some("token-fallback".into());
        let merged = dto().merge_into(base).unwrap();
        assert_eq!(merged.token.as_deref(), Some("token-fallback"));
    }

    #[test]
    fn project_key_duoc_chuan_hoa_hoa() {
        let mut d = dto();
        d.project_key = "  proj ".into();
        let m = d.merge_into(Config::default()).unwrap();
        assert_eq!(m.project_key, "PROJ");
    }

    #[test]
    fn tu_choi_url_sai() {
        let mut d = dto();
        d.jira_url = "atlassian.example.com".into();
        assert!(d.merge_into(Config::default()).is_err());

        let mut d2 = dto();
        d2.jira_url = "   ".into();
        assert!(d2.merge_into(Config::default()).is_err());
    }

    #[test]
    fn tu_choi_poll_qua_nhanh_va_board_rong() {
        let mut d = dto();
        d.poll_interval_secs = 3;
        assert!(d.merge_into(Config::default()).is_err());

        let mut d2 = dto();
        d2.board_id = 0;
        assert!(d2.merge_into(Config::default()).is_err());
    }

    #[test]
    fn tu_choi_nguong_am() {
        let mut d = dto();
        d.stale_days = -1;
        assert!(d.merge_into(Config::default()).is_err());
    }

    #[test]
    fn don_dep_danh_sach_status_rong() {
        let mut d = dto();
        d.review_statuses = vec!["  In Test ".into(), "".into(), "   ".into(), "Closed".into()];
        let m = d.merge_into(Config::default()).unwrap();
        assert_eq!(m.review_statuses, vec!["In Test", "Closed"]);
    }

    #[test]
    fn bo_trung_ke_ca_khi_khong_dung_canh_nhau() {
        let mut d = dto();
        d.review_statuses = vec!["In Test".into(), "Closed".into(), "In Test".into()];
        let m = d.merge_into(Config::default()).unwrap();
        assert_eq!(m.review_statuses, vec!["In Test", "Closed"]);
    }

    #[test]
    fn luu_cai_dat_khong_duoc_dong_vao_display_mode() {
        // Form co the da cu: nguoi dung bat Only Me tu tray SAU khi mo cua so
        // Cai dat. Neu `merge_into` ghi `display_mode` thi bam Luu se lang le
        // keo mode ve lai gia tri cu.
        let mut base = Config::default();
        base.display_mode = DisplayMode::OnlyMe;

        let mut d = dto(); // dto() sinh tu Config::default() -> "team"
        d.display_mode = "team".into();

        assert_eq!(
            d.merge_into(base).unwrap().display_mode,
            DisplayMode::OnlyMe,
            "mode dang chay phai thang form cu"
        );
    }

    #[test]
    fn me_rong_thi_ui_thay_team_du_file_ghi_only_me() {
        // AC-D5 nhin tu phia Cai dat: radio khong duoc hien "Chi viec cua toi"
        // trong khi panel that su dang chay o che do team.
        let mut c = Config::default();
        c.display_mode = DisplayMode::OnlyMe;
        c.me = String::new();
        assert_eq!(SettingsDto::from_config(&c, false).display_mode, "team");
    }

    #[test]
    fn window_layer_roundtrip() {
        let mut d = dto();
        d.window_layer = "floating".into();
        assert_eq!(
            d.merge_into(Config::default()).unwrap().window_layer,
            WindowLayer::Floating
        );
        let mut d2 = dto();
        d2.window_layer = "gi do la".into();
        assert_eq!(
            d2.merge_into(Config::default()).unwrap().window_layer,
            WindowLayer::Desktop,
            "gia tri la thi ve mac dinh chu khong panic"
        );
    }
}
