//! Vong poll 60s chay o Rust (AC-4) — van chay khi cua so bi an, vi no khong
//! phu thuoc vao webview. Kem backoff khi Jira khong voi toi duoc (AC-24).

use crate::config::{Config, DisplayMode};
use crate::diff::{diff, to_notifications};
use crate::jira::{JiraClient, JiraError, RawIssue, SprintMeta};
use crate::snapshot::{build, SprintSnapshot};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::{mpsc, Mutex};

pub const EVENT_STATE: &str = "panel://state";

/// Thang backoff khi loi lien tiep: 60s -> 2m -> 5m -> 10m (tran).
const BACKOFF_SECS: [u64; 4] = [60, 120, 300, 600];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelState {
    pub snapshot: Option<SprintSnapshot>,
    pub ok: bool,
    /// "auth" | "network" | "api" | "parse" — UI doi thong diep theo loai (AC-25)
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub last_success: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
    /// Giua hai sprint thi khong phai loi, chi la khong co gi de hien (AC-26)
    pub no_active_sprint: bool,
    pub poll_interval_secs: u64,
    /// UI can de ghi nhan "Dung im > N ngay" thay vi hardcode con so
    pub stale_days: i64,
    /// "vi" | "en" — de panel chon dung bo chuoi ngay tu lan render dau
    pub language: String,
}

pub struct AppState {
    pub cfg: Config,
    pub client: Arc<JiraClient>,
    pub inner: Mutex<Inner>,
    pub refresh_tx: mpsc::Sender<()>,
}

#[derive(Default)]
pub struct Inner {
    pub snapshot: Option<SprintSnapshot>,
    pub last_success: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub no_active_sprint: bool,
    /// Lan poll dau sau khi khoi dong: KHONG bao gi ca (AC-23)
    pub primed: bool,
    /// Mode dang ap dung. Song o day chu khong o `cfg` vi `cfg` bat bien sau
    /// khi khoi dong, con cai nay doi nong duoc.
    pub display_mode: DisplayMode,
    /// Ngon ngu dang ap dung — cung ly do voi `display_mode`: doi nong duoc.
    pub language: String,
    // --- Cache cua lan fetch gan nhat, chi de dung lai snapshot khi doi mode.
    // Doi mode ma phai cho toi 60 giay cho lan poll sau thi tinh nang vo dung,
    // nen giu nguyen lieu tho lai (AC-D2). 48 issue ~ 60KB, khong dang ke.
    pub last_raw: Vec<RawIssue>,
    pub last_sprint: Option<SprintMeta>,
}

impl Inner {
    pub fn new(display_mode: DisplayMode, language: String) -> Self {
        Self {
            display_mode,
            language,
            ..Default::default()
        }
    }
}

impl AppState {
    pub async fn panel_state(&self) -> PanelState {
        let g = self.inner.lock().await;
        PanelState {
            snapshot: g.snapshot.clone(),
            ok: g.error_kind.is_none(),
            error_kind: g.error_kind.clone(),
            error_message: g.error_message.clone(),
            last_success: g.last_success,
            consecutive_failures: g.consecutive_failures,
            no_active_sprint: g.no_active_sprint,
            poll_interval_secs: self.cfg.poll_interval_secs,
            stale_days: self.cfg.stale_days,
            language: g.language.clone(),
        }
    }
}

/// Dung lai snapshot tu cache raw, KHONG goi Jira (AC-D2, AC-D3).
///
/// Dung lai dung `last_success` lam moc thoi gian chu khong phai `Utc::now()`:
/// du lieu van la cua lan fetch cu, nen tuoi ticket va nhan "cap nhat luc ..."
/// phai giu nguyen. Lay `now` moi se lam so lieu nhich len ma khong co du lieu
/// moi nao dang sau.
pub async fn rebuild_from_cache(app: &AppHandle, state: &Arc<AppState>) -> bool {
    let t0 = std::time::Instant::now();

    // MOT lan khoa cho ca doc cache lan ghi snapshot. Tha khoa o giua thi mot
    // lan poll vua xong co the chen vao, roi ban dung lai tu cache CU se ghi de
    // len du lieu moi hon va giu cai cu do them 60 giay.
    //
    // Giu khoa qua `build()` an toan vi `build()` la tinh toan thuan tuy, khong
    // co `.await` nao ben trong nen khong the nhuong dieu phoi giua chung.
    let (n, mode) = {
        let mut g = state.inner.lock().await;
        let (Some(sprint), Some(at)) = (g.last_sprint.clone(), g.last_success) else {
            return false;
        };
        let (n, mode, raw) = (g.last_raw.len(), g.display_mode, g.last_raw.clone());
        g.snapshot = Some(build(raw, &sprint, &state.cfg, at, mode));
        (n, mode)
    };

    emit(app, state).await;
    // Ghi ra so that thay vi tin la "chac nhanh thoi": nguong de doi mode con
    // cam giac tuc thi la 300ms, va con so nay la cach duy nhat biet co vuot khong.
    log::info!(
        "dung lai snapshot tu cache ({n} issue, mode {}) trong {}ms",
        mode.as_str(),
        t0.elapsed().as_millis()
    );
    true
}

fn backoff(failures: u32, base: u64) -> Duration {
    if failures == 0 {
        return Duration::from_secs(base);
    }
    let idx = (failures as usize).min(BACKOFF_SECS.len() - 1);
    Duration::from_secs(BACKOFF_SECS[idx].max(base))
}

/// Cap nhat badge tren menu bar: so canh bao dang co.
///
/// O Only Me con so nay la canh bao CUA RIENG minh, vi `risks` duoc tinh sau
/// khi loc. Da can nhac cho no dem ca team: bo di, vi badge noi mot dang con
/// panel mo ra noi mot neo thi kho hieu hon la badge khop voi panel. Doi lai,
/// luc panel dang an thi khong co chip nao nhac rang man hinh dang bi loc —
/// day la cai gia da biet cua lua chon nay.
fn update_tray(app: &AppHandle, state: Option<&SprintSnapshot>, ok: bool) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    let title = match (ok, state) {
        (false, _) => " ⚠".to_string(),
        (true, Some(s)) if s.risks.count > 0 => format!(" {}", s.risks.count),
        _ => String::new(),
    };
    let _ = tray.set_title(Some(title));
}

async fn poll_once(app: &AppHandle, state: &Arc<AppState>) {
    let now = Utc::now();

    let sprint = match state.client.active_sprint(state.cfg.board_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            let mut g = state.inner.lock().await;
            g.no_active_sprint = true;
            g.error_kind = None;
            g.error_message = None;
            g.consecutive_failures = 0;
            drop(g);
            log::warn!("board {} chua co sprint dang chay", state.cfg.board_id);
            emit(app, state).await;
            return;
        }
        Err(e) => return record_error(app, state, e).await,
    };

    let raw = match state.client.sprint_issues(sprint.id).await {
        Ok(r) => r,
        Err(e) => return record_error(app, state, e).await,
    };

    let mut g = state.inner.lock().await;
    let next = build(raw.clone(), &sprint, &state.cfg, now, g.display_mode);
    let changes = match (g.primed, g.snapshot.as_ref()) {
        (true, Some(prev)) => diff(prev, &next),
        // Lan dau sau khi mo app: chi nap du lieu, khong bao (AC-23)
        _ => Vec::new(),
    };
    g.snapshot = Some(next);
    g.last_raw = raw;
    g.last_sprint = Some(sprint);
    g.last_success = Some(now);
    g.consecutive_failures = 0;
    g.error_kind = None;
    g.error_message = None;
    g.no_active_sprint = false;
    g.primed = true;
    drop(g);

    if !changes.is_empty() {
        log::info!("{} thay doi so voi lan poll truoc", changes.len());
        for n in to_notifications(&changes, &state.cfg.notify, &state.cfg.project_key) {
            if let Err(e) = app
                .notification()
                .builder()
                .title(&n.title)
                .body(&n.body)
                .show()
            {
                log::warn!("khong ban duoc notification: {e}");
            }
        }
    }

    emit(app, state).await;
}

async fn record_error(app: &AppHandle, state: &Arc<AppState>, e: JiraError) {
    {
        let mut g = state.inner.lock().await;
        g.consecutive_failures = g.consecutive_failures.saturating_add(1);
        g.error_kind = Some(e.kind().to_string());
        g.error_message = Some(e.to_string());
        log::warn!("poll loi lan {}: {e}", g.consecutive_failures);
    }
    // Khong xoa snapshot cu: panel van hien du lieu cuoi kem nhan tuoi du lieu (AC-24)
    emit(app, state).await;
}

async fn emit(app: &AppHandle, state: &Arc<AppState>) {
    let ps = state.panel_state().await;
    update_tray(app, ps.snapshot.as_ref(), ps.ok);
    if let Err(e) = app.emit(EVENT_STATE, &ps) {
        log::warn!("emit that bai: {e}");
    }
}

pub async fn run(app: AppHandle, state: Arc<AppState>, mut refresh_rx: mpsc::Receiver<()>) {
    let base = state.cfg.poll_interval_secs.max(10);
    loop {
        poll_once(&app, &state).await;

        let failures = state.inner.lock().await.consecutive_failures;
        let delay = backoff(failures, base);
        if failures > 0 {
            log::info!("thu lai sau {}s", delay.as_secs());
        }

        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            got = refresh_rx.recv() => {
                if got.is_none() { break; }
                // Refresh tay: ep resolve lai sprint thay vi doi cache 10 phut
                state.client.invalidate_sprint_cache().await;
                log::info!("refresh thu cong");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_leo_thang_roi_dung_o_10_phut() {
        assert_eq!(backoff(0, 60), Duration::from_secs(60));
        assert_eq!(backoff(1, 60), Duration::from_secs(120));
        assert_eq!(backoff(2, 60), Duration::from_secs(300));
        assert_eq!(backoff(3, 60), Duration::from_secs(600));
        assert_eq!(backoff(99, 60), Duration::from_secs(600), "co tran");
    }

    #[test]
    fn backoff_khong_bao_gio_nhanh_hon_chu_ky_cau_hinh() {
        // neu anh dat poll 300s thi luc loi khong duoc quay ve 60s
        assert_eq!(backoff(1, 300), Duration::from_secs(300));
        assert_eq!(backoff(3, 900), Duration::from_secs(900));
    }
}
