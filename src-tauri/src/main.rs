#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod diff;
mod jira;
mod oauth;
mod poller;
mod settings;
mod snapshot;

use config::{AuthMode, DisplayMode, WindowLayer};
use jira::JiraAuth;
use poller::{AppState, Inner, PanelState};
use std::sync::{Arc, OnceLock};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, LogicalSize, Manager, WebviewWindow};

/// Tray bao frontend lat che do compact (frontend giu trang thai that)
const EVENT_TOGGLE_COMPACT: &str = "panel://toggle-compact";
/// Mode vua doi — cua so Cai dat dang mo can biet de radio khong hien sai
const EVENT_DISPLAY_MODE: &str = "panel://display-mode";
/// Ngon ngu vua doi — moi webview dang mo tu doi chuoi, khong can khoi dong lai
const EVENT_LANGUAGE: &str = "panel://language";
/// Tray bat che do di chuyen — frontend hien banner "keo roi bam Xong"
const EVENT_MOVE_MODE: &str = "panel://move-mode";

/// Tich "Chi viec cua toi" tren tray. Giu tham chieu de con dong bo khi mode
/// bi doi tu cho khac (chip tren panel, radio trong Cai dat).
static MODE_ITEM: OnceLock<CheckMenuItem<tauri::Wry>> = OnceLock::new();
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as AutostartManagerExt};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::{mpsc, Mutex};

/// Ghim cua so xuong tang desktop.
///
/// Tauri khong co API cho viec nay nen phai goi thang NSWindow. Dung `msg_send!`
/// voi con tro tho thay vi binding objc2-app-kit: chi phu thuoc vao ten selector
/// cua AppKit (on dinh chuc nam nay) chu khong phu thuoc hinh dang API cua crate.
#[cfg(target_os = "macos")]
mod desktop_layer {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;

    // Hoi CoreGraphics con so that thay vi hardcode magic number:
    // level cua desktop icon hien la -2147483603, nhung day la chi tiet cai dat.
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGWindowLevelForKey(key: i32) -> i32;
    }
    const KEY_DESKTOP_ICON: i32 = 18; // kCGDesktopIconWindowLevelKey

    // NSWindowCollectionBehavior
    const CAN_JOIN_ALL_SPACES: isize = 1 << 0; // hien o moi Space
    const STATIONARY: isize = 1 << 4; // dung yen khi vao Mission Control
    const IGNORES_CYCLE: isize = 1 << 6; // khong nhay vao vong Cmd+`

    pub fn pin(ns_window: *mut std::ffi::c_void) -> i64 {
        if ns_window.is_null() {
            log::warn!("khong lay duoc NSWindow — bo qua viec ghim xuong desktop");
            return 0;
        }
        // +1 de nam ngay TREN icon desktop (panel de doc, khong nen bi icon che),
        // nhung van thap hon cua so app thuong (level 0) rat nhieu.
        let level = unsafe { CGWindowLevelForKey(KEY_DESKTOP_ICON) } as isize + 1;
        let behavior = CAN_JOIN_ALL_SPACES | STATIONARY | IGNORES_CYCLE;
        let obj = ns_window as *mut AnyObject;
        unsafe {
            let _: () = msg_send![obj, setLevel: level];
            let _: () = msg_send![obj, setCollectionBehavior: behavior];
        }
        level as i64
    }
}

// Thu gon theo he moi: header 1 dong (title + dem nguoc, ~39px) + strip tien
// do (~28px) + vien. Phai >= minHeight trong tauri.conf.json, khong thi
// set_size bi kep va panel khong thu xuong duoc.
const COMPACT_HEIGHT: f64 = 72.0;
const FULL_HEIGHT: f64 = 620.0;
const PANEL_WIDTH: f64 = 360.0;
const SCREEN_MARGIN: f64 = 16.0;

/// Nhan tray theo ngon ngu trong config. Tray dung chuoi tinh — doi ngon ngu
/// se ap dung sau lan khoi dong ke tiep (webview thi doi nong duoc).
fn tray_label(lang: &str, key: &str) -> &'static str {
    let en = lang == "en";
    match key {
        "show" => if en { "Show / hide panel" } else { "Hiện / ẩn panel" },
        "compact" => if en { "Collapse / expand" } else { "Thu gọn / mở rộng" },
        "onlyme" => if en { "Only my work" } else { "Chỉ việc của tôi" },
        "refresh" => if en { "Refresh now" } else { "Refresh ngay" },
        "move" => if en { "Move panel…" } else { "Di chuyển panel…" },
        "settings" => if en { "Settings…" } else { "Cài đặt…" },
        "autostart" => if en { "Start with macOS" } else { "Khởi động cùng macOS" },
        "quit" => if en { "Quit" } else { "Thoát" },
        _ => "",
    }
}

// ---------------------------------------------------------------- commands

#[tauri::command]
async fn get_state(state: tauri::State<'_, Arc<AppState>>) -> Result<PanelState, String> {
    Ok(state.panel_state().await)
}

#[tauri::command]
async fn refresh_now(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state
        .refresh_tx
        .send(())
        .await
        .map_err(|e| format!("khong gui duoc lenh refresh: {e}"))
}

/// Mo ticket bang browser mac dinh. Mo tu Rust chu khong tu webview de
/// capability cua webview giu nguyen muc toi thieu (`core:default`).
#[tauri::command]
fn open_issue(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let cfg_base = {
        let st = app.state::<Arc<AppState>>();
        st.cfg.jira_url.trim_end_matches('/').to_string()
    };
    // Chi cho mo link tro ve dung Jira instance — khong bien command nay
    // thanh cai cong mo URL tuy y. So sanh THEO GOC (scheme+host+port) chu
    // khong phai prefix chuoi: `https://jira.x.com.evil.com` va
    // `https://jira.x.com@evil.com` deu vuot qua duoc prefix check.
    if !jira::same_origin(&url, &cfg_base) {
        log::warn!("tu choi mo URL ngoai Jira: {url}");
        return Err(format!("tu choi mo URL ngoai Jira: {url}"));
    }
    log::info!("mo ticket: {url}");
    match app.opener().open_url(&url, None::<&str>) {
        Ok(()) => Ok(()),
        Err(e) => {
            log::error!("khong mo duoc browser cho {url}: {e}");
            Err(e.to_string())
        }
    }
}

#[tauri::command]
fn set_compact(app: tauri::AppHandle, compact: bool) -> Result<(), String> {
    let Some(win) = app.get_webview_window("main") else {
        return Ok(());
    };
    win.set_size(LogicalSize::new(
        PANEL_WIDTH,
        if compact { COMPACT_HEIGHT } else { FULL_HEIGHT },
    ))
    .map_err(|e| e.to_string())
}

/// Ap dung mode moi: nho vao config, dung lai snapshot tu cache, dong bo tick tray.
///
/// KHONG goi Jira va KHONG khoi dong lai — do la ca diem cua tinh nang nay
/// (AC-D2, AC-D3). Chi mot minh `display_mode` doi nong duoc; `project_key`,
/// `board_id`, token... van phai di qua "Luu & khoi dong lai" vi chung doi
/// ca client HTTP lan cache.
async fn apply_display_mode(
    app: &tauri::AppHandle,
    state: &Arc<AppState>,
    want: DisplayMode,
) -> Result<DisplayMode, String> {
    // muda lat tich cua CheckMenuItem TRUOC khi ban su kien, nen moi duong ra
    // khoi ham nay deu phai keo tich ve dung su that — ke ca duong loi va duong
    // "khong co gi de lam". Bo sot mot duong la tich hien nguoc voi thuc te.
    let sync_tick = |m: DisplayMode| {
        if let Some(item) = MODE_ITEM.get() {
            let _ = item.set_checked(m == DisplayMode::OnlyMe);
        }
    };

    let eff = want.effective_for(&state.cfg.me);
    if eff != want {
        sync_tick(state.inner.lock().await.display_mode);
        return Err("Chưa điền username ở Cài đặt → Phạm vi nên chưa bật được \
             “chỉ việc của tôi”. Nếu vừa điền thì bấm “Lưu & khởi động lại” đã."
            .into());
    }

    {
        let mut g = state.inner.lock().await;
        if g.display_mode == eff {
            sync_tick(eff); // bam lai dung cai dang chon: chi keo tich ve cho cu
            return Ok(eff);
        }
        g.display_mode = eff;
    }

    // Ghi xuong dia ngay de khoi dong lai van giu (AC-D4). Ghi hong thi mode
    // trong phien nay van dung, chi mat sau khi thoat — bao log chu khong chan.
    match config::load() {
        Ok(mut c) => {
            c.display_mode = eff;
            if let Err(e) = config::save(&c) {
                log::warn!("khong ghi duoc display_mode xuong config: {e}");
            }
        }
        Err(e) => log::warn!("khong doc duoc config de ghi display_mode: {e}"),
    }

    sync_tick(eff);
    let _ = app.emit(EVENT_DISPLAY_MODE, eff.as_str());

    if !poller::rebuild_from_cache(app, state).await {
        // Chua co lan fetch nao thanh cong -> khong co gi de dung lai. Lan poll
        // toi se tu dung dung mode moi.
        log::info!("doi mode khi chua co du lieu — cho lan poll toi");
    }
    log::info!("display mode -> {}", eff.as_str());
    Ok(eff)
}

#[tauri::command]
async fn set_display_mode(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    mode: String,
) -> Result<String, String> {
    let st = state.inner().clone();
    let eff = apply_display_mode(&app, &st, DisplayMode::from_str_or_team(&mode)).await?;
    Ok(eff.as_str().to_string())
}

/// Doi ngon ngu NONG: ghi config + bao moi webview dang mo. Khong dung toi
/// mang hay cache nen khong can khoi dong lai; rieng nhan tray la chuoi tinh,
/// se dung ngon ngu moi o lan khoi dong sau.
#[tauri::command]
async fn set_language(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    lang: String,
) -> Result<String, String> {
    let eff = config::normalize_lang(&lang);
    state.inner.lock().await.language = eff.to_string();

    match config::load() {
        Ok(mut c) => {
            c.language = eff.to_string();
            if let Err(e) = config::save(&c) {
                log::warn!("khong ghi duoc language xuong config: {e}");
            }
        }
        Err(e) => log::warn!("khong doc duoc config de ghi language: {e}"),
    }

    let _ = app.emit(EVENT_LANGUAGE, eff);
    log::info!("language -> {eff}");
    Ok(eff.to_string())
}

/// Mo cua so Cai dat tu panel (nut tren notice loi token chang han).
#[tauri::command]
fn settings_open(app: tauri::AppHandle) {
    settings::open_window(&app);
}

/// An panel xuong menu bar — nut ⤓ tren header. Hien lai bang icon tray.
#[tauri::command]
fn hide_panel(app: tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

/// Che do di chuyen: panel tam noi len tren de nhan chuot va keo tu do;
/// bam "Xong" thi tra ve dung tang da cau hinh (desktop hoac floating).
#[tauri::command]
fn set_move_mode(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    moving: bool,
) -> Result<(), String> {
    let Some(win) = app.get_webview_window("main") else {
        return Ok(());
    };
    if moving {
        let _ = win.set_always_on_top(true);
        let _ = win.show();
        let _ = win.set_focus();
        log::info!("bat che do di chuyen panel");
        return Ok(());
    }
    match state.cfg.window_layer {
        WindowLayer::Desktop => {
            let _ = win.set_always_on_top(false);
            #[cfg(target_os = "macos")]
            {
                match win.ns_window() {
                    Ok(ptr) => {
                        desktop_layer::pin(ptr);
                    }
                    Err(e) => log::warn!("khong lay duoc NSWindow khi tra panel ve desktop: {e}"),
                }
            }
        }
        WindowLayer::Floating => {
            let _ = win.set_always_on_top(true);
        }
    }
    log::info!("tat che do di chuyen — panel ve tang {:?}", state.cfg.window_layer);
    Ok(())
}

#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enable: bool) -> Result<(), String> {
    let al = app.autolaunch();
    if enable {
        al.enable().map_err(|e| e.to_string())
    } else {
        al.disable().map_err(|e| e.to_string())
    }
}

// ----------------------------------------------------------------- helpers

fn toggle_window(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    match win.is_visible() {
        Ok(true) => {
            let _ = win.hide();
        }
        _ => {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

/// Lan dau chay thi ghim panel vao goc tren ben phai man hinh chinh.
/// Nhung lan sau de plugin window-state khoi phuc vi tri anh da keo.
fn place_top_right_if_first_run(app: &tauri::AppHandle, win: &WebviewWindow) {
    let saved = app
        .path()
        .app_config_dir()
        .map(|d| d.join(".window-state.json"))
        .map(|p| p.exists())
        .unwrap_or(false);
    if saved {
        return;
    }
    let Ok(Some(monitor)) = win.primary_monitor() else {
        return;
    };
    let scale = monitor.scale_factor();
    let size = monitor.size().to_logical::<f64>(scale);
    let pos = monitor.position().to_logical::<f64>(scale);
    let x = pos.x + size.width - PANEL_WIDTH - SCREEN_MARGIN;
    let y = pos.y + SCREEN_MARGIN + 28.0; // chua cho menu bar
    let _ = win.set_position(tauri::LogicalPosition::new(x, y));
}

fn build_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let lang = config::normalize_lang(&app.state::<Arc<AppState>>().cfg.language).to_string();
    let tl = |k: &str| tray_label(&lang, k);

    let show = MenuItem::with_id(app, "show", tl("show"), true, None::<&str>)?;
    let prefs = MenuItem::with_id(app, "settings", tl("settings"), true, None::<&str>)?;
    let compact = MenuItem::with_id(app, "compact", tl("compact"), true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", tl("refresh"), true, None::<&str>)?;
    let move_item = MenuItem::with_id(app, "move", tl("move"), true, None::<&str>)?;
    let project = app.state::<Arc<AppState>>().cfg.project_key.clone();
    let board_label = if lang == "en" {
        format!("Open board {project}")
    } else {
        format!("Mở board {project}")
    };
    let board = MenuItem::with_id(app, "board", &board_label, true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    // Only Me chi co nghia khi biet "toi" la ai — chua dien username thi item
    // nay xam di thay vi bat duoc roi ra panel rong (AC-D5).
    let co_me = !app.state::<Arc<AppState>>().cfg.me.trim().is_empty();
    let dang_only_me =
        app.state::<Arc<AppState>>().cfg.effective_display_mode() == DisplayMode::OnlyMe;
    let onlyme = CheckMenuItem::with_id(
        app,
        "onlyme",
        tl("onlyme"),
        co_me,
        dang_only_me,
        None::<&str>,
    )?;
    let _ = MODE_ITEM.set(onlyme.clone());

    let autostart_on = app.autolaunch().is_enabled().unwrap_or(false);
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        tl("autostart"),
        true,
        autostart_on,
        None::<&str>,
    )?;

    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", tl("quit"), true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show, &compact, &onlyme, &refresh, &move_item, &board, &sep, &prefs, &autostart,
            &sep2, &quit,
        ],
    )?;

    // Giu tham chieu de tich lai o dung trang thai that sau khi bat/tat
    let autostart_item = autostart.clone();

    let tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().expect("co icon mac dinh").clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(&format!("Master Jira — {project}"))
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => toggle_window(app),
            "settings" => settings::open_window(app),
            // Frontend giu trang thai compact, nen tray chi bao hieu roi de no tu lat.
            // Neu tray tu resize thi hai ben lech nhau ngay lan bam thu hai.
            "compact" => {
                if let Err(e) = app.emit(EVENT_TOGGLE_COMPACT, ()) {
                    log::warn!("khong gui duoc lenh thu gon: {e}");
                }
            }
            // Frontend hien banner + goi set_move_mode(true); Rust chi bao hieu.
            "move" => {
                if let Err(e) = app.emit(EVENT_MOVE_MODE, ()) {
                    log::warn!("khong gui duoc lenh di chuyen: {e}");
                }
            }
            "autostart" => {
                let al = app.autolaunch();
                let dang_bat = al.is_enabled().unwrap_or(false);
                let ket_qua = if dang_bat { al.disable() } else { al.enable() };
                match ket_qua {
                    Ok(()) => {
                        let _ = autostart_item.set_checked(!dang_bat);
                        log::info!("autostart -> {}", if dang_bat { "tat" } else { "bat" });
                    }
                    // Tich phai bam theo su that: doi that bai thi giu nguyen tich cu
                    Err(e) => {
                        log::warn!("khong doi duoc autostart: {e}");
                        let _ = autostart_item.set_checked(dang_bat);
                    }
                }
            }
            "onlyme" => {
                let state = app.state::<Arc<AppState>>().inner().clone();
                let handle = app.clone();
                // Doc trang thai that tu Inner chu khong tu tick cua menu: tick
                // co the da bi doi tu chip hay Cai dat ma menu chua ve lai.
                tauri::async_runtime::spawn(async move {
                    let dang = state.inner.lock().await.display_mode;
                    let muon = match dang {
                        DisplayMode::OnlyMe => DisplayMode::Team,
                        DisplayMode::Team => DisplayMode::OnlyMe,
                    };
                    if let Err(e) = apply_display_mode(&handle, &state, muon).await {
                        log::warn!("khong doi duoc display mode: {e}");
                        // Doi hong thi tra tick ve dung su that
                        if let Some(item) = MODE_ITEM.get() {
                            let _ = item.set_checked(dang == DisplayMode::OnlyMe);
                        }
                    }
                });
            }
            "refresh" => {
                let state = app.state::<Arc<AppState>>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = state.refresh_tx.send(()).await;
                });
            }
            "board" => {
                let st = app.state::<Arc<AppState>>();
                let url = format!(
                    "{}/secure/RapidBoard.jspa?rapidView={}",
                    st.cfg.jira_url.trim_end_matches('/'),
                    st.cfg.board_id
                );
                let _ = app.opener().open_url(url, None::<&str>);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    // Icon don sac theo theme menu bar
    let _ = tray.set_icon_as_template(true);
    Ok(())
}

// -------------------------------------------------------------------- main

/// Log ra file, khong phai stderr: app mo bang Finder thi stderr di vao hu vo,
/// nen khi co su co (bam ticket khong mo browser, token het han) khong co gi de doc.
fn init_logging() {
    let mut b = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("jira_widget=info"),
    );
    if let Ok(home) = std::env::var("HOME") {
        let dir = std::path::PathBuf::from(home).join("Library").join("Logs");
        let path = dir.join("jira-widget.log");
        let _ = std::fs::create_dir_all(&dir);
        match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(f) => {
                b.target(env_logger::Target::Pipe(Box::new(f)));
            }
            Err(e) => eprintln!("khong mo duoc log file {}: {e}", path.display()),
        }
    }
    b.init();
}

fn print_help() {
    println!(
        "Master Jira\n\n\
         jira-widget                 chay panel\n\
         jira-widget --set-token     nap Jira PAT cua ban vao Keychain (doc tu stdin)\n\
         jira-widget --clear-token   xoa token khoi Keychain\n\
         jira-widget --list-boards [PROJECT]\n\
         \x20                                liet ke board + id de dien vao config\n\n\
         Config: ~/Library/Application Support/jira-widget/config.toml\n\
         Log:    ~/Library/Logs/jira-widget.log"
    );
}

/// Liet ke board cua project — buoc khong the thieu khi dung config lan dau,
/// vi Jira khong hien board id o bat ky dau tren giao dien.
fn list_boards(project: Option<&str>) -> anyhow::Result<()> {
    let cfg = config::load()?;
    let key = project
        .map(str::to_string)
        .unwrap_or_else(|| cfg.project_key.clone());
    if key.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "chua biet project nao. Chay: jira-widget --list-boards <PROJECT>"
        ));
    }
    let client = match cfg.auth_mode {
        AuthMode::DcPat => jira::JiraClient::new_pat(&cfg.jira_url, config::resolve_token(&cfg)?)?,
        AuthMode::CloudBasic => jira::JiraClient::new(
            &cfg.jira_url,
            None,
            JiraAuth::Basic {
                email: cfg.email.trim().to_string(),
                token: config::resolve_token(&cfg)?,
            },
        )?,
        AuthMode::CloudOauth => {
            return Err(anyhow::anyhow!(
                "config dang o che do OAuth — dung wizard/Cai dat trong app de chon board"
            ));
        }
    };
    let rt = tokio::runtime::Runtime::new()?;
    let boards = rt.block_on(client.list_boards(&key))?;

    if boards.is_empty() {
        println!("Project {key} khong co board nao (hoac token khong thay duoc).");
        return Ok(());
    }
    println!("Board cua project {key}:\n");
    println!("  {:<8} {:<10} {}", "id", "loai", "ten");
    for b in &boards {
        println!("  {:<8} {:<10} {}", b.id, b.board_type, b.name);
    }
    println!("\nDien so o cot `id` vao `board_id` trong config.toml.");
    Ok(())
}

fn main() {
    init_logging();

    let args: Vec<String> = std::env::args().collect();
    let cli = match args.get(1).map(|s| s.as_str()) {
        Some("--set-token") => Some(config::set_token_from_stdin()),
        Some("--clear-token") => Some(config::clear_token()),
        Some("--list-boards") => Some(list_boards(args.get(2).map(|s| s.as_str()))),
        Some("--help") | Some("-h") => {
            print_help();
            std::process::exit(0);
        }
        _ => None,
    };
    if let Some(res) = cli {
        match res {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("Loi: {e:#}");
                std::process::exit(1);
            }
        }
    }

    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Khong doc duoc config: {e:#}");
            std::process::exit(1);
        }
    };
    // Dung auth theo mode. Thieu credential thi KHONG chet lang le — dau panel
    // di va mo wizard; client van duoc dung (voi bi mat rong) de moi request
    // tra ve Auth va panel hien dung state neu wizard bi dong ngang.
    let (auth, api_base, thieu_token) = match cfg.auth_mode {
        AuthMode::DcPat => {
            let token = config::resolve_token(&cfg).unwrap_or_else(|e| {
                log::warn!("{e}");
                String::new()
            });
            let thieu = token.is_empty();
            (JiraAuth::Pat(token), None, thieu)
        }
        AuthMode::CloudBasic => {
            let token = config::resolve_token(&cfg).unwrap_or_else(|e| {
                log::warn!("{e}");
                String::new()
            });
            let thieu = token.is_empty() || cfg.email.trim().is_empty();
            (
                JiraAuth::Basic {
                    email: cfg.email.trim().to_string(),
                    token,
                },
                None,
                thieu,
            )
        }
        AuthMode::CloudOauth => {
            let thieu = !oauth::has_refresh_token() || cfg.cloud_id.trim().is_empty();
            let api = (!cfg.cloud_id.trim().is_empty())
                .then(|| oauth::api_base_for(&cfg.cloud_id));
            // Store DUNG CHUNG voi cac lenh probe/whoami — refresh token xoay
            // vong nen ca process chi duoc phep co MOT nguoi giu no.
            match oauth::shared_store(&cfg) {
                Ok(store) => (JiraAuth::Oauth(store), api, thieu),
                Err(e) => {
                    // Chua cau hinh backend OAuth ma config lai ghi cloud_oauth
                    // — coi nhu chua dang nhap, de wizard/Cai dat xu ly.
                    log::warn!("khong dung duoc OAuth store: {e:#}");
                    (JiraAuth::Pat(String::new()), api, true)
                }
            }
        }
    };
    let client = match jira::JiraClient::new(&cfg.jira_url, api_base, auth) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("Khong khoi tao duoc HTTP client: {e:#}");
            std::process::exit(1);
        }
    };

    let (refresh_tx, refresh_rx) = mpsc::channel::<()>(4);
    // Doc mode tu config NHUNG da qua `effective_`: file bi sua tay thanh
    // only_me trong khi `me` rong thi ve team, khong de panel rong (AC-D5).
    let mode = cfg.effective_display_mode();
    let lang = config::normalize_lang(&cfg.language).to_string();
    let state = Arc::new(AppState {
        cfg,
        client,
        inner: Mutex::new(Inner::new(mode, lang)),
        refresh_tx,
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(state.clone())
        .setup(move |app| {
            // Panel song tren menu bar, khong chiem cho o Dock
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            if let Some(win) = app.get_webview_window("main") {
                match state.cfg.window_layer {
                    WindowLayer::Desktop => {
                        // always_on_top phai TAT truoc: neu bat, AppKit ghi de level
                        // minh vua dat va panel lai noi len tren.
                        let _ = win.set_always_on_top(false);
                        #[cfg(target_os = "macos")]
                        {
                            match win.ns_window() {
                                Ok(ptr) => {
                                    let lv = desktop_layer::pin(ptr);
                                    log::info!("panel ghim vao desktop (NSWindow level {lv})");
                                }
                                Err(e) => log::warn!("khong lay duoc NSWindow: {e}"),
                            }
                        }
                    }
                    WindowLayer::Floating => {
                        let _ = win.set_visible_on_all_workspaces(true);
                        let _ = win.set_always_on_top(true);
                        log::info!("panel o che do noi tren cung");
                    }
                }
                place_top_right_if_first_run(app.handle(), &win);
            }

            build_tray(app)?;

            // Lan dau (chua co token): dau panel di — dang sau wizard ma lo mot
            // panel bao loi ket noi thi rat de tuong app hong — roi mo wizard
            // dan tung buoc. Nguoi dung cu du thong tin thi khong thay gi ca.
            if thieu_token {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide();
                }
                settings::open_onboarding(app.handle());
            }

            let handle = app.handle().clone();
            let st = state.clone();
            tauri::async_runtime::spawn(async move {
                poller::run(handle, st, refresh_rx).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            refresh_now,
            open_issue,
            set_compact,
            set_display_mode,
            set_language,
            settings_open,
            hide_panel,
            set_move_mode,
            get_autostart,
            set_autostart,
            settings::settings_get,
            settings::settings_save,
            settings::settings_save_token,
            settings::settings_clear_token,
            settings::settings_test_connection,
            settings::settings_list_boards,
            settings::settings_project_statuses,
            settings::settings_apply_restart,
            settings::settings_close,
            settings::onboarding_finish,
            settings::oauth_begin,
            settings::oauth_whoami,
            settings::oauth_logout
        ])
        .run(tauri::generate_context!())
        .expect("khong khoi dong duoc app");
}
