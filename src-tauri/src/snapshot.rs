//! Bien raw issue tu Jira thanh `SprintSnapshot` — moi con so hien tren panel
//! deu duoc tinh o day. Pure function, khong cham mang, khong cham thoi gian he
//! thong (`now` truyen vao) nen test duoc bang fixture co dinh.

use crate::config::{Config, DisplayMode};
use crate::jira::{parse_jira_datetime, RawIssue, SprintMeta};
use chrono::{DateTime, Duration as ChronoDuration, FixedOffset, Utc};
use serde::Serialize;

/// Gio Viet Nam. Khong co DST nen FixedOffset la du, khoi keo ca tz database.
pub fn vn_offset() -> FixedOffset {
    FixedOffset::east_opt(7 * 3600).expect("offset +07:00 hop le")
}

/// Khoang cach tinh theo NGAY LICH o gio VN, khong phai duration chia 86400.
///
/// Ly do: anh doc "ticket nay song 55 ngay" theo nghia lich. Neu lay duration
/// truncate thi ticket tao luc 16h ngay 29/05 xem luc 05h ngay 23/07 se ra 54,
/// lech 1 ngay so voi cach nguoi doc dem.
pub fn calendar_days_between(from: DateTime<Utc>, to: DateTime<Utc>) -> i64 {
    let tz = vn_offset();
    (to.with_timezone(&tz).date_naive() - from.with_timezone(&tz).date_naive()).num_days()
}

// ------------------------------------------------------------------- models

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub key: String,
    pub summary: String,
    pub status: String,
    /// "new" | "indeterminate" | "done"
    pub status_category: String,
    pub assignee: Option<String>,
    pub assignee_display: Option<String>,
    pub initials: String,
    /// Ten goi de doc, vd `Tuan` — thay cho chu viet tat kho hieu
    pub short_name: String,
    pub issue_type: String,
    pub priority: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    /// created -> now, theo ngay lich (AC-20)
    pub age_days: i64,
    /// updated -> now, theo ngay lich
    pub idle_days: i64,
    pub story_point: Option<f64>,
    pub app_task_score: Option<f64>,
    pub url: String,
    pub is_open: bool,
    pub is_pending_release: bool,
    pub is_stale: bool,
    pub is_old: bool,
    /// `me` nam trong Approvers (cf_10200) — vai tro DUYET
    pub is_approver_me: bool,
    /// `me` nam trong QCs (cf_10201) — vai tro TEST.
    /// Tach rieng vi hai vai tro nay la hai nguoi khac nhau: tren project
    /// Approvers = alex.lee (leader), QCs = blake.kim (nguoi test).
    pub is_qc_me: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub total: usize,
    /// Jira coi la Done — GOM ca Ready for Release
    pub done: usize,
    /// Done that su (da tru pending release)
    pub closed: usize,
    /// Jira tinh Done nhung thuc te moi cho release (AC-14)
    pub pending_release: usize,
    pub in_progress: usize,
    pub todo: usize,
    pub percent: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Risks {
    pub stale: Vec<Issue>,
    pub ending_soon: Vec<Issue>,
    pub unassigned: Vec<Issue>,
    /// true khi sprint con it hon `ending_soon_hours` — quyet dinh ending_soon
    /// co duoc tinh vao badge canh bao hay khong
    pub sprint_ending_soon: bool,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StatusCount {
    pub status: String,
    pub count: usize,
}

/// Tinh tren TOAN BO ticket cua sprint, khong chi ticket dang mo.
///
/// Ban dau chi dem ticket dang mo — sai nghiem trong: member da dong het viec
/// bien mat khoi panel (casey.park lam 10 ticket, nhieu nhat sprint, khong hien
/// mot dong nao), va donut bao "44%" cho nguoi thuc ra chiem 20% khoi luong.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemberLoad {
    pub name: String,
    pub display: String,
    pub initials: String,
    /// Ten goi de doc, vd `Tuan`
    pub short: String,
    /// Toan bo ticket trong sprint
    pub total: usize,
    pub done: usize,
    pub open: usize,
    pub in_progress: usize,
    pub todo: usize,
    pub done_percent: u32,
    pub sp_sum: f64,
    pub score_sum: f64,
    pub by_status: Vec<StatusCount>,
    /// Ticket chua giao ai — gom thanh mot dong rieng, khong cap mau dinh danh
    pub is_unassigned: bool,
    /// Dong nay la chinh anh (`config.me`) — to dam de de tim
    pub is_me: bool,
}

/// Tong diem cua MOT pham vi. Luon di kem mau so nen UI khong the hien Σ tran (F2).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PointScope {
    pub sp_sum: f64,
    pub sp_filled: usize,
    pub score_sum: f64,
    pub score_filled: usize,
    pub denominator: usize,
}

/// Hai pham vi song song. Truoc day chi co mot, tinh tren ticket dang mo, nen
/// "Σ SP 6.5" doc nham thanh diem ca sprint trong khi no bo qua het ticket da dong.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PointTotals {
    /// Ca 46 ticket cua sprint
    pub sprint: PointScope,
    /// Chi ticket chua xong — tap con cua `sprint`
    pub open: PointScope,
}

/// Mot hang doi cho viec. Mang theo luon pham vi va co an/hien de UI khong
/// phai tu suy ra tu cho khac.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub items: Vec<Issue>,
    /// "mine" khi da loc theo `me`, "all" khi khong loc
    pub scope: String,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgeStats {
    pub median_age: i64,
    pub max_age: i64,
    pub median_idle: i64,
    pub max_idle: i64,
}

/// Nguoi dang xem panel — chi co khi `config.me` khong rong.
/// Dung cho chip "● Tuan · chi viec cua toi" tren header (AC-D12).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Viewer {
    pub name: String,
    /// Ten goi de doc, vd `Tuan`
    pub short: String,
    pub display: String,
}

/// Ban rut gon cua MOT ticket, chi du de biet "co gi doi so voi lan poll truoc".
///
/// Ton tai vi diff phai chay tren CA sprint bat ke dang xem mode nao (AC-D18):
/// ticket bi doi assignee sang nguoi khac se roi khoi tap Only Me, ma do lai
/// chinh la thay doi member can biet nhat. Giu ban day du thu hai thi ton bo
/// nho va IPC vo ich, nen chi giu 5 field diff thuc su doc toi.
#[derive(Debug, Clone, PartialEq)]
pub struct IssueDigest {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub assignee: Option<String>,
    pub assignee_display: Option<String>,
}

impl From<&Issue> for IssueDigest {
    fn from(i: &Issue) -> Self {
        Self {
            key: i.key.clone(),
            summary: i.summary.clone(),
            status: i.status.clone(),
            assignee: i.assignee.clone(),
            assignee_display: i.assignee_display.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SprintSnapshot {
    pub fetched_at: DateTime<Utc>,
    pub sprint_id: u64,
    pub sprint_name: String,
    pub sprint_end: Option<DateTime<Utc>>,
    /// Giay con lai toi het sprint; am nghia la da qua han
    pub seconds_left: Option<i64>,
    /// Da loc theo mode. O `OnlyMe` chi con ticket cua `me`.
    pub issues: Vec<Issue>,
    pub open_issues: Vec<Issue>,
    pub progress: Progress,
    /// Tien do CA sprint, LUON tinh trươc khi loc — o Team mode no trung
    /// `progress`, o Only Me no la dong "ca sprint 37/46 · 80%" (AC-D8).
    pub sprint_context: Progress,
    pub display_mode: DisplayMode,
    pub viewer: Option<Viewer>,
    /// Username xep theo so ticket giam dan, tinh tren CA sprint. Frontend cap
    /// mau theo danh sach nay nen mau khong doi khi bat/tat Only Me.
    pub color_order: Vec<String>,
    pub risks: Risks,
    pub by_assignee: Vec<MemberLoad>,
    /// Cho test — loc theo QCs
    pub test_queue: Queue,
    /// Cho duyet — loc theo Approvers
    pub review_queue: Queue,
    /// Cho release — khong loc theo nguoi
    pub release_queue: Queue,
    pub points: PointTotals,
    pub age_stats: AgeStats,
    /// KHONG gui qua IPC — chi de `diff()` so hai lan poll tren toan sprint.
    #[serde(skip)]
    pub all_digest: Vec<IssueDigest>,
}

// --------------------------------------------------------------- helpers

/// `alex.lee` -> `TN`. Uu tien username vi no on dinh va ngan;
/// display name kieu "Alex Lee - Engineering" cho chu cai xau.
pub fn initials_of(name: Option<&str>, display: Option<&str>) -> String {
    if let Some(n) = name.filter(|s| !s.is_empty()) {
        let parts: Vec<String> = n
            .split(|c: char| c == '.' || c == '_' || c == '-')
            .filter(|p| !p.is_empty())
            .map(|p| {
                p.chars()
                    .filter(|c| c.is_alphabetic())
                    .take(1)
                    .collect::<String>()
                    .to_uppercase()
            })
            .filter(|p| !p.is_empty())
            .collect();
        if !parts.is_empty() {
            return parts.into_iter().take(2).collect();
        }
    }
    if let Some(d) = display.filter(|s| !s.is_empty()) {
        let letters: String = d
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .filter(|c| c.is_alphabetic())
            .take(2)
            .collect();
        if !letters.is_empty() {
            return letters.to_uppercase();
        }
    }
    "?".into()
}

/// Khoa kieu accountId cua Jira Cloud — khong phai username doc duoc.
/// Dang `557058:f58131cb-...` (co dau `:`) hoac chuoi hex dai
/// `5b10ac8d82e05b22cc7d4ef5`. Username DC khong bao gio co dau `:`.
fn la_account_id(u: &str) -> bool {
    u.contains(':') || (u.len() >= 20 && u.chars().all(|c| c.is_ascii_hexdigit()))
}

/// `alex.lee` -> `Tuan`. Ban gon cho khoa la username that (DC) — giu lai cho
/// test va cho fallback khong co display name.
pub fn short_name_map(usernames: &[String]) -> std::collections::HashMap<String, String> {
    let pairs: Vec<(String, Option<String>)> =
        usernames.iter().map(|u| (u.clone(), None)).collect();
    short_name_map_display(&pairs)
}

/// `alex.lee` -> `Tuan`.
///
/// Truoc day cot nay hien chu viet tat (`TN`, `ND`, `KN`...) — nhin vao phai
/// giai ma tung cai, khong doc duoc. Ten goi thi nhan ra ngay.
///
/// Neu hai nguoi trung ten goi (rat hay gap: hai ban cung ten Tuan) thi them
/// chu cai ho: `Tuan N.` / `Tuan L.`; van trung nua thi tra ve dinh danh day du
/// — tha dai con hon chi sai nguoi.
///
/// Khoa kieu accountId (Cloud) khong rut ten goi tu khoa duoc — moi buoc lay
/// tu DISPLAY name di kem: ten goi = tu dau, chu cai ho = tu thu hai, fallback
/// cuoi = display day du (da cat hau to " - Phong ban").
pub fn short_name_map_display(
    pairs: &[(String, Option<String>)],
) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;

    let seg = |u: &str, i: usize| -> String {
        u.split(|c: char| c == '.' || c == '_' || c == '-')
            .filter(|p| !p.is_empty())
            .nth(i)
            .unwrap_or("")
            .chars()
            .filter(|c| c.is_alphabetic())
            .collect()
    };
    let cap = |s: &str| -> String {
        let mut c = s.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    };
    // Tu display "Gale Shaw - Engineering" lay word thu `i` cua phan ten.
    let dword = |d: &str, i: usize| -> String {
        shorten_display(d)
            .split_whitespace()
            .nth(i)
            .unwrap_or("")
            .chars()
            .filter(|c| c.is_alphabetic())
            .collect()
    };

    // buoc 1: ten goi
    let mut label: HashMap<String, String> = HashMap::new();
    for (u, d) in pairs {
        let goi = if la_account_id(u) {
            d.as_deref().map(|x| cap(&dword(x, 0)))
        } else {
            Some(cap(&seg(u, 0)))
        };
        let goi = goi.filter(|s| !s.is_empty());
        label.insert(u.clone(), goi.unwrap_or_else(|| u.clone()));
    }

    // buoc 2: trung thi them chu cai ho
    let dup = |m: &HashMap<String, String>| -> HashMap<String, usize> {
        let mut c: HashMap<String, usize> = HashMap::new();
        for v in m.values() {
            *c.entry(v.clone()).or_insert(0) += 1;
        }
        c
    };
    let counts = dup(&label);
    for (u, d) in pairs {
        let cur = label[u].clone();
        if counts.get(&cur).copied().unwrap_or(0) > 1 {
            let ho = if la_account_id(u) {
                d.as_deref().map(|x| dword(x, 1)).unwrap_or_default()
            } else {
                seg(u, 1)
            };
            if let Some(ch) = ho.chars().next() {
                label.insert(u.clone(), format!("{cur} {}.", ch.to_uppercase()));
            }
        }
    }

    // buoc 3: van trung -> DC dung han username; Cloud dung han display name
    let counts2 = dup(&label);
    for (u, d) in pairs {
        let cur = label[u].clone();
        if counts2.get(&cur).copied().unwrap_or(0) > 1 {
            let full = if la_account_id(u) {
                d.as_deref()
                    .map(shorten_display)
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| u.clone())
            } else {
                u.clone()
            };
            label.insert(u.clone(), full);
        }
    }
    label
}

/// "Gale Shaw - Engineering" -> "Gale Shaw"
fn shorten_display(display: &str) -> String {
    display
        .split(" - ")
        .next()
        .unwrap_or(display)
        .trim()
        .to_string()
}

/// Ten gia cho nhom ticket chua giao ai. Khong the trung username that vi
/// Jira khong cho dau gach duoi kep o dau ten.
pub const UNASSIGNED: &str = "__chua_giao__";

fn point_scope(list: &[Issue]) -> PointScope {
    PointScope {
        sp_sum: list.iter().filter_map(|i| i.story_point).sum(),
        sp_filled: list.iter().filter(|i| i.story_point.is_some()).count(),
        score_sum: list.iter().filter_map(|i| i.app_task_score).sum(),
        score_filled: list.iter().filter(|i| i.app_task_score.is_some()).count(),
        denominator: list.len(),
    }
}

/// Tien do cua MOT tap ticket. Tach ra ham rieng vi bay gio phai tinh hai lan:
/// mot cho tap dang hien, mot cho ca sprint lam boi canh (AC-D8).
fn progress_of(list: &[Issue]) -> Progress {
    let total = list.len();
    let done = list.iter().filter(|i| !i.is_open).count();
    let pending_release = list
        .iter()
        .filter(|i| !i.is_open && i.is_pending_release)
        .count();
    Progress {
        total,
        done,
        closed: done.saturating_sub(pending_release),
        pending_release,
        in_progress: list
            .iter()
            .filter(|i| i.status_category == "indeterminate")
            .count(),
        todo: list.iter().filter(|i| i.status_category == "new").count(),
        percent: if total == 0 {
            0
        } else {
            ((done as f64 / total as f64) * 100.0).round() as u32
        },
    }
}

fn median(sorted: &[i64]) -> i64 {
    if sorted.is_empty() {
        0
    } else {
        sorted[sorted.len() / 2]
    }
}

// ----------------------------------------------------------------- compute

pub fn build(
    raw: Vec<RawIssue>,
    sprint: &SprintMeta,
    cfg: &Config,
    now: DateTime<Utc>,
    mode: DisplayMode,
) -> SprintSnapshot {
    let base = cfg.jira_url.trim_end_matches('/');
    let me = cfg.me.trim();
    // Mode di vao qua tham so chu khong doc thang `cfg.display_mode`: nguoi dung
    // doi mode NONG qua tray/chip, luc do config trong bo nho da cu.
    let mode = mode.effective_for(me);
    // Mot board co the chua issue cua nhieu project. Loc theo prefix key de
    // panel chi hien dung project anh phu trach.
    let prefix = if cfg.project_key.trim().is_empty() {
        String::new()
    } else {
        format!("{}-", cfg.project_key.trim())
    };

    let mut issues: Vec<Issue> = Vec::with_capacity(raw.len());
    for r in raw {
        if !prefix.is_empty() && !r.key.starts_with(&prefix) {
            continue;
        }
        let created = match parse_jira_datetime(&r.fields.created) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("{}: bo qua vi created loi — {e}", r.key);
                continue;
            }
        };
        let updated = parse_jira_datetime(&r.fields.updated).unwrap_or(created);

        let cat = r.fields.status.category.key.to_lowercase();
        let is_open = cat != "done";
        let status = r.fields.status.name.clone();
        let is_pending_release = cfg.pending_release_statuses.iter().any(|s| s == &status);

        // Khoa dinh danh: `name` (DC) else `accountId` (Cloud). Moi phep so
        // khop — `me`, mau sac, gom nhom — deu chay tren khoa nay.
        let assignee = r.fields.assignee.as_ref().and_then(|u| u.username());
        // Ten that chi DC co. Initials phai suy tu no (hoac display), KHONG
        // duoc suy tu khoa: khoa Cloud la accountId, rut chu cai ra rac.
        let assignee_real_name = r.fields.assignee.as_ref().and_then(|u| u.name.clone());
        let assignee_display = r
            .fields
            .assignee
            .as_ref()
            .and_then(|u| u.display_name.clone());

        let age_days = calendar_days_between(created, now);
        let idle_days = calendar_days_between(updated, now);

        issues.push(Issue {
            initials: initials_of(assignee_real_name.as_deref(), assignee_display.as_deref()),
            short_name: String::new(), // dien o pass 2, khi da biet het assignee
            key: r.key.clone(),
            url: format!("{base}/browse/{}", r.key),
            summary: r.fields.summary.clone(),
            status,
            status_category: cat,
            assignee,
            assignee_display,
            issue_type: r
                .fields
                .issuetype
                .as_ref()
                .map(|t| t.name.clone())
                .unwrap_or_else(|| "Task".into()),
            priority: r
                .fields
                .priority
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "-".into()),
            created,
            updated,
            age_days,
            idle_days,
            story_point: r.fields.story_point,
            app_task_score: r.fields.app_task_score,
            is_open,
            is_pending_release,
            // stale/is_old chi co y nghia voi ticket dang mo
            is_stale: is_open && idle_days > cfg.stale_days,
            is_old: is_open && age_days >= cfg.old_age_days,
            is_approver_me: !me.is_empty()
                && crate::jira::usernames_from(&r.fields.approvers)
                    .iter()
                    .any(|u| u == me),
            is_qc_me: !me.is_empty()
                && crate::jira::usernames_from(&r.fields.qcs).iter().any(|u| u == me),
        });
    }

    // Ticket moi dong nhat len dau danh sach chinh; rieng cac nhom risk sap xep rieng.
    issues.sort_by(|a, b| b.updated.cmp(&a.updated));

    // Pass 2 — ten goi phai biet TOAN BO assignee moi phat hien duoc trung ten,
    // nen khong the tinh trong vong lap dung tung ticket o tren. Mang theo ca
    // display name: khoa kieu accountId (Cloud) chi rut duoc ten goi tu display.
    let mut pairs: Vec<(String, Option<String>)> = Vec::new();
    for i in &issues {
        if let Some(u) = &i.assignee {
            if !pairs.iter().any(|(k, _)| k == u) {
                pairs.push((u.clone(), i.assignee_display.clone()));
            }
        }
    }
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    let short = short_name_map_display(&pairs);
    for i in issues.iter_mut() {
        i.short_name = match &i.assignee {
            Some(u) => short.get(u).cloned().unwrap_or_else(|| u.clone()),
            None => "chua giao".into(),
        };
    }

    // ==================== MOC LOC THEO MODE ====================
    // Tu day tro xuong `issues` co the chi con viec cua mot nguoi. Moi thu can
    // ca sprint phai duoc tinh O TREN moc nay, khong duoc dat xuong duoi.

    // Boi canh sprint: luon la ca sprint, ke ca dang o Only Me (AC-D8)
    let sprint_context = progress_of(&issues);
    // Co so cho diff — cung phai la ca sprint (AC-D18)
    let all_digest: Vec<IssueDigest> = issues.iter().map(IssueDigest::from).collect();

    let viewer = if me.is_empty() {
        None
    } else {
        let mine = issues.iter().find(|i| i.assignee.as_deref() == Some(me));
        Some(Viewer {
            name: me.to_string(),
            // Co ticket thi lay dung ten goi da tinh tren toan bo assignee (co
            // the co hau to chong trung); khong co thi tu suy tu username.
            // Rieng `me` kieu accountId (Cloud) ma khong co ticket nao thi
            // khong co gi doc duoc de suy — tra RONG cho frontend tu thay
            // bang chu "tôi"/"me", khong hien khoa tho.
            short: mine.map(|i| i.short_name.clone()).unwrap_or_else(|| {
                if la_account_id(me) {
                    String::new()
                } else {
                    short_name_map(&[me.to_string()])
                        .get(me)
                        .cloned()
                        .unwrap_or_else(|| me.to_string())
                }
            }),
            display: mine
                .and_then(|i| i.assignee_display.as_deref())
                .map(shorten_display)
                .unwrap_or_else(|| me.to_string()),
        })
    };

    // Thu tu cap mau, tinh tren CA SPRINT.
    //
    // Neu de frontend suy tu `by_assignee` (duoi moc) thi o Only Me danh sach do
    // chi con MOT nguoi: ticket cua nguoi khac trong ba hang doi mat cham mau,
    // va mau cua chinh minh nhay sang series dau tien. Mau phai bam theo NGUOI,
    // khong bam theo viec hom nay dang loc ai.
    let color_order: Vec<String> = {
        let mut dem: Vec<(String, usize)> = Vec::new();
        for i in issues.iter().filter_map(|i| i.assignee.as_ref()) {
            match dem.iter().position(|(n, _)| n == i) {
                Some(k) => dem[k].1 += 1,
                None => dem.push((i.clone(), 1)),
            }
        }
        // Cung quy tac sap xep voi `by_assignee` de hai ben khong lech nhau
        dem.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        dem.into_iter().map(|(n, _)| n).collect()
    };

    // Ba hang doi tinh trên TOAN SPRINT o ca hai mode (AC-D17).
    //
    // Chung loc theo VAI TRO cua minh (Approvers / QCs), khong theo nguoi lam:
    // ticket cho minh duyet hay minh test gan nhu luon la viec cua nguoi khac.
    // Loc them theo assignee thi ba hang doi nay rong sach o Only Me — dung
    // luc member can chung nhat.
    //
    // Status quyet dinh "dang cho gi", field nguoi quyet dinh "cho AI".
    // Test va Duyet la HAI vai tro khac nhau nen loc theo hai field khac nhau —
    // ban truoc gop `approvers ∪ qcs` lam mot, tron lan nguoi test voi nguoi duyet.
    // Release khong loc theo nguoi: do la viec cua ca team.
    let build_queue = |statuses: &[String], visible: bool, mine: Option<&dyn Fn(&Issue) -> bool>| {
        let mut items: Vec<Issue> = issues
            .iter()
            .filter(|i| statuses.iter().any(|s| s == &i.status))
            .filter(|i| match mine {
                Some(f) if !me.is_empty() => f(i),
                _ => true,
            })
            .cloned()
            .collect();
        items.sort_by(|a, b| b.idle_days.cmp(&a.idle_days).then_with(|| a.key.cmp(&b.key)));
        Queue {
            items,
            scope: if mine.is_some() && !me.is_empty() { "mine".into() } else { "all".into() },
            visible,
        }
    };

    let test_queue = build_queue(
        &cfg.test_statuses,
        cfg.show_test_queue,
        Some(&|i: &Issue| i.is_qc_me),
    );
    let review_queue = build_queue(
        &cfg.review_statuses,
        cfg.show_review_queue,
        Some(&|i: &Issue| i.is_approver_me),
    );
    let release_queue = build_queue(&cfg.pending_release_statuses, cfg.show_release_queue, None);

    // Ticket chua giao ai khong thuoc ve ai ca -> `Some(me)` loai no ra (AC-D10)
    if mode == DisplayMode::OnlyMe {
        issues.retain(|i| i.assignee.as_deref() == Some(me));
    }
    // ================== HET PHAN TINH TREN CA SPRINT ==================

    let open_issues: Vec<Issue> = issues.iter().filter(|i| i.is_open).cloned().collect();

    // -------- progress (AC-14): tach Ready for Release ra khoi Closed
    let progress = progress_of(&issues);

    // -------- countdown
    let seconds_left = sprint.end.map(|e| (e - now).num_seconds());
    let sprint_ending_soon = seconds_left
        .map(|s| s > 0 && s < ChronoDuration::hours(cfg.ending_soon_hours).num_seconds())
        .unwrap_or(false);

    // -------- risks (AC-13)
    let mut stale: Vec<Issue> = open_issues.iter().filter(|i| i.is_stale).cloned().collect();
    stale.sort_by(|a, b| b.idle_days.cmp(&a.idle_days));

    let unassigned: Vec<Issue> = open_issues
        .iter()
        .filter(|i| i.assignee.is_none())
        .cloned()
        .collect();

    let mut ending_soon = open_issues.clone();
    ending_soon.sort_by(|a, b| b.idle_days.cmp(&a.idle_days));

    let count = stale.len()
        + unassigned.len()
        + if sprint_ending_soon { ending_soon.len() } else { 0 };

    let risks = Risks {
        stale,
        ending_soon,
        unassigned,
        sprint_ending_soon,
        count,
    };

    // -------- tai theo member (AC-S1..S3) — tinh tren tap DA LOC (duoi moc).
    // Van la TOAN BO ticket cua tap do, ke ca da dong: dem moi ticket dang mo
    // thi member dong het viec bien mat khoi panel.
    let mut by_assignee: Vec<MemberLoad> = Vec::new();
    for i in &issues {
        let unassigned = i.assignee.is_none();
        let name = i.assignee.clone().unwrap_or_else(|| UNASSIGNED.to_string());

        // Tra ve index chu khong phai &mut: giu `iter_mut().find()` roi push trong
        // nhanh None se vuong borrow checker (muon &mut hai lan tren cung vec).
        let idx = match by_assignee.iter().position(|m| m.name == name) {
            Some(i) => i,
            None => {
                by_assignee.push(MemberLoad {
                    display: if unassigned {
                        "chua giao".to_string()
                    } else {
                        i.assignee_display
                            .as_deref()
                            .map(shorten_display)
                            .unwrap_or_else(|| name.clone())
                    },
                    initials: if unassigned { "--".into() } else { i.initials.clone() },
                    short: if unassigned { "chua giao".into() } else { i.short_name.clone() },
                    name: name.clone(),
                    total: 0,
                    done: 0,
                    open: 0,
                    in_progress: 0,
                    todo: 0,
                    done_percent: 0,
                    sp_sum: 0.0,
                    score_sum: 0.0,
                    by_status: Vec::new(),
                    is_unassigned: unassigned,
                    is_me: !me.is_empty() && name == me,
                });
                by_assignee.len() - 1
            }
        };
        let entry = &mut by_assignee[idx];
        entry.total += 1;
        if i.is_open {
            entry.open += 1;
        } else {
            entry.done += 1;
        }
        match i.status_category.as_str() {
            "indeterminate" => entry.in_progress += 1,
            "new" => entry.todo += 1,
            _ => {}
        }
        entry.sp_sum += i.story_point.unwrap_or(0.0);
        entry.score_sum += i.app_task_score.unwrap_or(0.0);
        match entry.by_status.iter_mut().find(|s| s.status == i.status) {
            Some(s) => s.count += 1,
            None => entry.by_status.push(StatusCount {
                status: i.status.clone(),
                count: 1,
            }),
        }
    }
    for m in by_assignee.iter_mut() {
        m.done_percent = if m.total == 0 {
            0
        } else {
            ((m.done as f64 / m.total as f64) * 100.0).round() as u32
        };
        m.by_status
            .sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.status.cmp(&b.status)));
    }
    // "chua giao" luon xuong cuoi; con lai nhieu viec nhat len dau, hoa thi theo
    // ten cho thu tu on dinh giua cac lan poll.
    by_assignee.sort_by(|a, b| {
        a.is_unassigned
            .cmp(&b.is_unassigned)
            .then_with(|| b.total.cmp(&a.total))
            .then_with(|| a.name.cmp(&b.name))
    });

    // -------- diem so (AC-S6..S8) — hai pham vi, moi cai kem mau so rieng
    let points = PointTotals {
        sprint: point_scope(&issues),
        open: point_scope(&open_issues),
    };

    // -------- age stats (AC-20)
    let mut ages: Vec<i64> = open_issues.iter().map(|i| i.age_days).collect();
    let mut idles: Vec<i64> = open_issues.iter().map(|i| i.idle_days).collect();
    ages.sort_unstable();
    idles.sort_unstable();
    let age_stats = AgeStats {
        median_age: median(&ages),
        max_age: ages.last().copied().unwrap_or(0),
        median_idle: median(&idles),
        max_idle: idles.last().copied().unwrap_or(0),
    };

    SprintSnapshot {
        fetched_at: now,
        sprint_id: sprint.id,
        sprint_name: sprint.name.clone(),
        sprint_end: sprint.end,
        seconds_left,
        issues,
        open_issues,
        progress,
        sprint_context,
        display_mode: mode,
        viewer,
        color_order,
        risks,
        by_assignee,
        test_queue,
        review_queue,
        release_queue,
        points,
        age_stats,
        all_digest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jira::{RawFields, RawNamed, RawStatus, RawStatusCategory, RawUser};

    fn now_fixture() -> DateTime<Utc> {
        // 2026-07-23 12:00 gio VN
        parse_jira_datetime("2026-07-23T12:00:00.000+0700").unwrap()
    }

    fn sprint_fixture() -> SprintMeta {
        SprintMeta {
            id: 9302,
            name: "Sprint 24/07/2026".into(),
            board_id: 1000,
            start: parse_jira_datetime("2026-07-06T14:50:00.000+0700").ok(),
            end: parse_jira_datetime("2026-07-24T19:50:00.000+0700").ok(),
        }
    }

    fn issue(
        key: &str,
        status: &str,
        cat: &str,
        assignee: Option<(&str, &str)>,
        created: &str,
        updated: &str,
        sp: Option<f64>,
        score: Option<f64>,
    ) -> RawIssue {
        RawIssue {
            key: key.into(),
            fields: RawFields {
                summary: format!("{key} summary"),
                status: RawStatus {
                    name: status.into(),
                    category: RawStatusCategory { key: cat.into() },
                },
                assignee: assignee.map(|(n, d)| RawUser {
                    name: Some(n.into()),
                    account_id: None,
                    display_name: Some(d.into()),
                }),
                created: created.into(),
                updated: updated.into(),
                issuetype: Some(RawNamed { name: "Task".into() }),
                priority: Some(RawNamed {
                    name: "Medium".into(),
                }),
                story_point: sp,
                app_task_score: score,
                approvers: None,
                qcs: None,
            },
        }
    }

    /// Dung dung 9 ticket dang mo cua Sprint 24/07/2026 (khao sat 2026-07-23).
    fn fixture_9_ticket_that() -> Vec<RawIssue> {
        vec![
            issue("PROJ-1910", "Pending", "new", Some(("gale.shaw", "Gale Shaw - Engineering")),
                  "2026-05-29T16:00:20.000+0700", "2026-07-07T17:53:22.000+0700", Some(3.0), Some(3.0)),
            issue("PROJ-1999", "Open", "new", Some(("drew.cruz", "Drew Cruz - Engineering")),
                  "2026-06-22T14:15:53.000+0700", "2026-07-07T17:53:22.000+0700", None, Some(2.0)),
            issue("PROJ-2041", "Pending", "new", Some(("blake.kim", "Blake Kim - Engineering")),
                  "2026-06-30T14:45:03.000+0700", "2026-07-07T17:53:22.000+0700", Some(1.0), None),
            issue("PROJ-2054", "In Test", "indeterminate", Some(("alex.lee", "Alex Lee - Engineering")),
                  "2026-07-03T17:12:10.000+0700", "2026-07-16T16:34:08.000+0700", None, None),
            issue("PROJ-2038", "In Progress", "indeterminate", Some(("alex.lee", "Alex Lee - Engineering")),
                  "2026-06-29T14:34:15.000+0700", "2026-07-22T10:15:07.000+0700", Some(1.0), Some(2.0)),
            issue("PROJ-2046", "In Test", "indeterminate", Some(("alex.lee", "Alex Lee - Engineering")),
                  "2026-07-01T16:40:36.000+0700", "2026-07-22T10:16:35.000+0700", Some(0.5), None),
            issue("PROJ-2070", "In Progress", "indeterminate", Some(("alex.lee", "Alex Lee - Engineering")),
                  "2026-07-10T10:14:00.000+0700", "2026-07-22T10:41:21.000+0700", None, None),
            issue("PROJ-2077", "Ready for Test", "indeterminate", Some(("finn.reed", "Finn Reed - Engineering")),
                  "2026-07-16T10:49:06.000+0700", "2026-07-22T15:42:46.000+0700", Some(1.0), None),
            issue("PROJ-2104", "Ready for Test", "indeterminate", Some(("evan.diaz", "Evan Diaz - Engineering")),
                  "2026-07-22T14:26:35.000+0700", "2026-07-22T14:30:35.000+0700", None, None),
        ]
    }

    fn snap() -> SprintSnapshot {
        build(
            fixture_9_ticket_that(),
            &sprint_fixture(),
            &Config::default(),
            now_fixture(),
            DisplayMode::Team,
        )
    }

    #[test]
    fn age_khop_voi_cach_nguoi_dem_theo_lich() {
        let s = snap();
        let get = |k: &str| s.issues.iter().find(|i| i.key == k).unwrap().clone();
        // PROJ-1910 tao 29/05, xem 23/07 -> 55 ngay lich
        assert_eq!(get("PROJ-1910").age_days, 55);
        assert_eq!(get("PROJ-1999").age_days, 31);
        assert_eq!(get("PROJ-2041").age_days, 23);
        assert_eq!(get("PROJ-2054").age_days, 20);
        assert_eq!(get("PROJ-2104").age_days, 1);
    }

    #[test]
    fn idle_dung() {
        let s = snap();
        let get = |k: &str| s.issues.iter().find(|i| i.key == k).unwrap().clone();
        assert_eq!(get("PROJ-1910").idle_days, 16);
        assert_eq!(get("PROJ-2054").idle_days, 7);
        assert_eq!(get("PROJ-2104").idle_days, 1);
    }

    #[test]
    fn stale_nguong_3_ngay_ra_4_ticket() {
        // 1910/1999/2041 dung im 16 ngay + 2054 dung im 7 ngay.
        // (Ban nhap plan ghi 3 — sai, vi bo sot PROJ-2054 @7d > 3d.)
        let s = snap();
        let keys: Vec<&str> = s.risks.stale.iter().map(|i| i.key.as_str()).collect();
        assert_eq!(s.risks.stale.len(), 4, "stale = {keys:?}");
        assert!(keys.contains(&"PROJ-2054"));
        // sap xep giam dan theo idle
        assert_eq!(s.risks.stale[0].idle_days, 16);
        assert_eq!(s.risks.stale[3].key, "PROJ-2054");
    }

    #[test]
    fn nguong_stale_doi_duoc_qua_config() {
        let mut cfg = Config::default();
        cfg.stale_days = 10;
        let s = build(fixture_9_ticket_that(), &sprint_fixture(), &cfg, now_fixture(), DisplayMode::Team);
        assert_eq!(s.risks.stale.len(), 3, "nguong 10 ngay thi 2054 @7d khong con stale");
    }

    #[test]
    fn khong_co_ticket_thieu_assignee() {
        assert_eq!(snap().risks.unassigned.len(), 0);
    }

    /// Dung lai CA sprint 9302 nhu doi soat that ngay 2026-07-23:
    /// 46 ticket = 9 dang mo + 37 da dong, trai tren 7 nguoi + 1 ticket chua giao.
    /// So lieu lay tu JQL `sprint = 9302 AND assignee = ...` tung nguoi.
    fn fixture_ca_sprint() -> Vec<RawIssue> {
        let mut v = fixture_9_ticket_that(); // 9 ticket dang mo

        // (username, display, so ticket DA DONG)
        let da_dong: [(&str, &str, usize); 7] = [
            ("casey.park", "Casey Park - Engineering", 10),
            ("blake.kim", "Blake Kim - Engineering", 8),
            ("alex.lee", "Alex Lee - Engineering", 5),
            ("drew.cruz", "Drew Cruz - Engineering", 5),
            ("evan.diaz", "Evan Diaz - Engineering", 4),
            ("finn.reed", "Finn Reed - Engineering", 2),
            ("gale.shaw", "Gale Shaw - Engineering", 2),
        ];

        let mut n = 0;
        for (user, display, count) in da_dong {
            for k in 0..count {
                n += 1;
                v.push(issue(
                    &format!("PROJ-D{n}"),
                    if k == 0 { "Ready for Release" } else { "Closed" },
                    "done",
                    Some((user, display)),
                    "2026-07-08T10:00:00.000+0700",
                    "2026-07-20T10:00:00.000+0700",
                    // moi nguoi cho 1 ticket dong co SP 0.5 -> tong sprint > tong dang mo
                    if k == 0 { Some(0.5) } else { None },
                    None,
                ));
            }
        }
        // PROJ-1998: da dong, KHONG giao ai, co app task score 2.0
        v.push(issue(
            "PROJ-1998", "Closed", "done", None,
            "2026-06-20T10:00:00.000+0700", "2026-07-15T10:00:00.000+0700", None, Some(2.0),
        ));
        v
    }

    fn snap_ca_sprint() -> SprintSnapshot {
        build(
            fixture_ca_sprint(),
            &sprint_fixture(),
            &Config::default(),
            now_fixture(),
            DisplayMode::Team,
        )
    }

    /// Bien mot RawIssue thanh "dang cho `who` duyet".
    fn voi_approver(mut r: RawIssue, who: &str) -> RawIssue {
        r.fields.approvers = Some(serde_json::json!([{ "name": who }]));
        r
    }

    #[test]
    fn loc_theo_project_key_bo_issue_cua_project_khac() {
        // Mot board co the chua issue cua nhieu project — khong loc thi panel
        // cong ca ticket khong thuoc pham vi anh phu trach.
        let mut raw = fixture_9_ticket_that();
        raw.push(issue("APPBOT-1", "Open", "new", Some(("ai.do", "Ai Do")),
                       "2026-07-10T10:00:00.000+0700", "2026-07-22T10:00:00.000+0700", None, None));
        let s = build(raw, &sprint_fixture(), &Config::default(), now_fixture(), DisplayMode::Team);
        assert_eq!(s.issues.len(), 9, "APPBOT-1 phai bi loai");
        assert!(!s.issues.iter().any(|i| i.key.starts_with("APPBOT")));
    }

    #[test]
    fn project_key_rong_thi_khong_loc_gi() {
        let mut cfg = Config::default();
        cfg.project_key = String::new();
        let mut raw = fixture_9_ticket_that();
        raw.push(issue("APPBOT-1", "Open", "new", None,
                       "2026-07-10T10:00:00.000+0700", "2026-07-22T10:00:00.000+0700", None, None));
        let s = build(raw, &sprint_fixture(), &cfg, now_fixture(), DisplayMode::Team);
        assert_eq!(s.issues.len(), 10);
    }

    #[test]
    fn me_rong_thi_hang_doi_duyet_la_cua_ca_team() {
        // Khong biet minh la ai -> khong loc theo nguoi, hien ca hang doi.
        let s = snap();
        assert_eq!(s.review_queue.scope, "all");
        assert!(Config::default().me.is_empty());
        assert!(!s.by_assignee.iter().any(|m| m.is_me));
    }

    #[test]
    fn chi_ticket_dung_status_moi_vao_hang_doi_duyet() {
        // Day la loi cu: ticket `Open`/`In Progress` ma minh la Approver van bi
        // keo vao "cho toi duyet", trong khi no con dang duoc code.
        let mut cfg = Config::default();
        cfg.me = "sam.hale".into();
        cfg.review_statuses = vec!["Ready for Review".into()];

        let mut raw = fixture_9_ticket_that();
        // PROJ-1910 dang `Pending` — minh la Approver nhung CHUA cho duyet
        raw[0] = voi_approver(raw[0].clone(), "sam.hale");
        let s = build(raw, &sprint_fixture(), &cfg, now_fixture(), DisplayMode::Team);
        assert!(
            s.review_queue.items.is_empty(),
            "ticket chua toi buoc duyet thi khong duoc vao hang doi"
        );
    }

    #[test]
    fn dung_status_va_dung_nguoi_duyet_thi_moi_vao() {
        let mut cfg = Config::default();
        cfg.me = "sam.hale".into();
        cfg.review_statuses = vec!["Ready for Review".into()];

        let mut raw = fixture_9_ticket_that();
        raw.push(voi_approver(
            issue("PROJ-3000", "Ready for Review", "indeterminate",
                  Some(("evan.diaz", "Evan Diaz - Engineering")),
                  "2026-07-10T10:00:00.000+0700", "2026-07-20T10:00:00.000+0700", None, None),
            "sam.hale",
        ));
        // cung status nhung nguoi duyet la nguoi khac
        raw.push(voi_approver(
            issue("PROJ-3001", "Ready for Review", "indeterminate",
                  Some(("evan.diaz", "Evan Diaz - Engineering")),
                  "2026-07-10T10:00:00.000+0700", "2026-07-20T10:00:00.000+0700", None, None),
            "alex.lee",
        ));
        let s = build(raw, &sprint_fixture(), &cfg, now_fixture(), DisplayMode::Team);
        assert_eq!(s.review_queue.scope, "mine");
        assert_eq!(s.review_queue.items.len(), 1);
        assert_eq!(s.review_queue.items[0].key, "PROJ-3000");
    }

    #[test]
    fn field_nguoi_doc_duoc_ca_hai_dang_du_lieu() {
        let mut cfg = Config::default();
        cfg.me = "sam.hale".into();
        cfg.review_statuses = vec!["Ready for Review".into()];
        // Jira co the tra ve mang chuoi hoac mang object tuy cau hinh field
        for shape in [
            serde_json::json!(["sam.hale"]),
            serde_json::json!([{ "name": "sam.hale", "displayName": "Sam Hale" }]),
        ] {
            let mut raw = fixture_9_ticket_that();
            let mut extra = issue("PROJ-3002", "Ready for Review", "indeterminate",
                                  Some(("evan.diaz", "Evan Diaz")),
                                  "2026-07-10T10:00:00.000+0700",
                                  "2026-07-20T10:00:00.000+0700", None, None);
            extra.fields.approvers = Some(shape.clone());
            raw.push(extra);
            let s = build(raw, &sprint_fixture(), &cfg, now_fixture(), DisplayMode::Team);
            assert_eq!(s.review_queue.items.len(), 1, "dang du lieu: {shape}");
        }
    }

    #[test]
    fn nguoi_test_va_nguoi_duyet_la_hai_vai_tro_khac_nhau() {
        // Da kiem chung tren PROJ-2104: Approvers = alex.lee (leader duyet),
        // QCs = blake.kim (nguoi test). Ban truoc gop `approvers ∪ qcs` lam mot
        // nen leader bi keo vao hang doi test va nguoc lai.
        let mut raw = fixture_9_ticket_that();
        let mut t = issue("PROJ-4000", "Ready for Test", "indeterminate",
                          Some(("evan.diaz", "Evan Diaz")),
                          "2026-07-10T10:00:00.000+0700",
                          "2026-07-20T10:00:00.000+0700", None, None);
        t.fields.approvers = Some(serde_json::json!(["alex.lee"]));
        t.fields.qcs = Some(serde_json::json!(["blake.kim"]));
        raw.push(t);

        // Leader: thay o hang doi DUYET, KHONG thay o hang doi TEST
        let mut leader = Config::default();
        leader.me = "alex.lee".into();
        leader.test_statuses = vec!["Ready for Test".into()];
        leader.review_statuses = vec!["Ready for Test".into()]; // cung status, khac vai tro
        let s1 = build(raw.clone(), &sprint_fixture(), &leader, now_fixture(), DisplayMode::Team);
        assert_eq!(s1.review_queue.items.len(), 1, "leader phai thay viec can duyet");
        assert!(s1.test_queue.items.is_empty(), "leader khong phai QC, khong vao hang doi test");

        // QC: nguoc lai
        let mut qc = Config::default();
        qc.me = "blake.kim".into();
        qc.test_statuses = vec!["Ready for Test".into()];
        qc.review_statuses = vec!["Ready for Test".into()];
        let s2 = build(raw, &sprint_fixture(), &qc, now_fixture(), DisplayMode::Team);
        assert_eq!(s2.test_queue.items.len(), 1, "QC phai thay viec can test");
        assert!(s2.review_queue.items.is_empty(), "QC khong phai nguoi duyet");
    }

    #[test]
    fn hang_doi_release_khong_loc_theo_nguoi() {
        // Release la viec cua ca team, khong cua rieng ai.
        let mut cfg = Config::default();
        cfg.me = "khong.ai.ca".into();
        let mut raw = fixture_9_ticket_that();
        raw.push(issue("PROJ-5000", "Ready for Release", "done",
                       Some(("evan.diaz", "Evan Diaz")),
                       "2026-07-10T10:00:00.000+0700",
                       "2026-07-20T10:00:00.000+0700", None, None));
        let s = build(raw, &sprint_fixture(), &cfg, now_fixture(), DisplayMode::Team);
        assert_eq!(s.release_queue.items.len(), 1);
        assert_eq!(s.release_queue.scope, "all");
    }

    #[test]
    fn tat_hien_thi_thi_queue_van_tinh_nhung_bao_visible_false() {
        // Giu du lieu de bat lai la thay ngay, khong phai cho lan poll sau.
        let mut cfg = Config::default();
        cfg.show_test_queue = false;
        cfg.test_statuses = vec!["Ready for Test".into()];
        let s = build(fixture_9_ticket_that(), &sprint_fixture(), &cfg, now_fixture(), DisplayMode::Team);
        assert!(!s.test_queue.visible);
        assert_eq!(s.test_queue.items.len(), 2, "van tinh du lieu");
        assert!(s.review_queue.visible);
    }

    #[test]
    fn dong_cua_chinh_minh_duoc_danh_dau() {
        let mut cfg = Config::default();
        cfg.me = "alex.lee".into();
        let s = build(fixture_ca_sprint(), &sprint_fixture(), &cfg, now_fixture(), DisplayMode::Team);
        let me_rows: Vec<&MemberLoad> = s.by_assignee.iter().filter(|m| m.is_me).collect();
        assert_eq!(me_rows.len(), 1);
        assert_eq!(me_rows[0].name, "alex.lee");
    }

    #[test]
    fn fixture_ca_sprint_khop_con_so_that() {
        let s = snap_ca_sprint();
        assert_eq!(s.issues.len(), 46, "sprint 9302 co 46 ticket");
        assert_eq!(s.open_issues.len(), 9);
        assert_eq!(s.progress.done, 37);
    }

    #[test]
    fn member_dong_het_viec_van_phai_xuat_hien() {
        // Day la bug goc: casey.park lam 10 ticket, nhieu nhat sprint, dong het
        // -> bien mat hoan toan khoi panel khi chi dem ticket dang mo.
        let s = snap_ca_sprint();
        let ngan = s
            .by_assignee
            .iter()
            .find(|m| m.name == "casey.park")
            .expect("casey.park phai co trong danh sach");
        assert_eq!(ngan.total, 10);
        assert_eq!(ngan.done, 10);
        assert_eq!(ngan.open, 0);
        assert_eq!(ngan.done_percent, 100);
        assert_eq!(s.by_assignee[0].name, "casey.park", "nhieu viec nhat len dau");
    }

    #[test]
    fn moi_member_co_tong_va_so_da_done() {
        let s = snap_ca_sprint();
        let get = |n: &str| s.by_assignee.iter().find(|m| m.name == n).unwrap().clone();

        let tuan = get("alex.lee");
        assert_eq!((tuan.total, tuan.done, tuan.open), (9, 5, 4));
        assert_eq!(tuan.done_percent, 56); // 5/9 = 55.6 -> 56

        let vinh = get("blake.kim");
        assert_eq!((vinh.total, vinh.done, vinh.open), (9, 8, 1));

        let chien = get("drew.cruz");
        assert_eq!((chien.total, chien.done, chien.open), (6, 5, 1));

        let dat = get("evan.diaz");
        assert_eq!((dat.total, dat.done, dat.open), (5, 4, 1));

        for n in ["finn.reed", "gale.shaw"] {
            let m = get(n);
            assert_eq!((m.total, m.done, m.open), (3, 2, 1), "{n}");
        }
    }

    #[test]
    fn ticket_chua_giao_thanh_mot_dong_rieng_o_cuoi() {
        let s = snap_ca_sprint();
        assert_eq!(s.by_assignee.len(), 8, "7 nguoi + 1 dong chua giao");
        let last = s.by_assignee.last().unwrap();
        assert!(last.is_unassigned);
        assert_eq!(last.display, "chua giao");
        assert_eq!(last.total, 1);
        // dung cuoi bang du co nguoi it viec hon no khong
        assert!(!s.by_assignee[0].is_unassigned);
    }

    #[test]
    fn bat_bien_tong_done_cua_member_bang_progress_done() {
        // Neu hai con so nay lech nhau thi mot trong hai dang tinh sai pham vi.
        let s = snap_ca_sprint();
        let tong: usize = s.by_assignee.iter().map(|m| m.done).sum();
        assert_eq!(tong, s.progress.done, "tong done tung nguoi phai bang tong sprint");
        let tong_all: usize = s.by_assignee.iter().map(|m| m.total).sum();
        assert_eq!(tong_all, s.issues.len());
    }

    #[test]
    fn bat_bien_diem_ca_sprint_khong_bao_gio_nho_hon_phan_chua_xong() {
        for s in [snap(), snap_ca_sprint()] {
            assert!(s.points.sprint.sp_sum >= s.points.open.sp_sum);
            assert!(s.points.sprint.score_sum >= s.points.open.score_sum);
            assert!(s.points.sprint.denominator >= s.points.open.denominator);
        }
    }

    #[test]
    fn diem_ca_sprint_gom_ca_ticket_da_dong() {
        let s = snap_ca_sprint();
        // dang mo: 6.5 (nhu cu) + 7 ticket dong moi nguoi 0.5 = 10.0
        assert_eq!(s.points.open.sp_sum, 6.5);
        assert_eq!(s.points.sprint.sp_sum, 10.0);
        assert_eq!(s.points.sprint.denominator, 46);
        // score: dang mo 7.0, cong PROJ-1998 (chua giao, da dong) 2.0 = 9.0
        assert_eq!(s.points.open.score_sum, 7.0);
        assert_eq!(s.points.sprint.score_sum, 9.0);
    }

    #[test]
    fn nguoi_om_nhieu_viec_nhat_len_dau() {
        let s = snap();
        assert_eq!(s.by_assignee[0].name, "alex.lee");
        assert_eq!(s.by_assignee[0].total, 4);
        assert_eq!(s.by_assignee[0].initials, "AL");
        assert_eq!(s.by_assignee[0].display, "Alex Lee");
        // 2 In Test + 2 In Progress deu la indeterminate
        assert_eq!(s.by_assignee[0].in_progress, 4);
        assert_eq!(s.by_assignee.len(), 6, "6 nguoi dang giu viec");
    }

    #[test]
    fn diem_so_luon_kem_mau_so() {
        let s = snap();
        assert_eq!(s.points.open.sp_sum, 6.5); // 3.0 + 1.0 + 1.0 + 0.5
        assert_eq!(s.points.open.sp_filled, 5);
        assert_eq!(s.points.open.score_sum, 7.0); // 3.0 + 2.0 + 2.0
        assert_eq!(s.points.open.score_filled, 3);
        assert_eq!(s.points.open.denominator, 9);
        assert!(
            s.points.open.sp_filled < s.points.open.denominator,
            "du lieu that dang thua — UI bat buoc phai hien mau so"
        );
    }

    #[test]
    fn review_queue_lay_theo_config_khong_hardcode() {
        let mut cfg = Config::default();
        cfg.review_statuses = vec!["Ready for Test".into(), "In Test".into()];
        let s = build(fixture_9_ticket_that(), &sprint_fixture(), &cfg, now_fixture(), DisplayMode::Team);
        assert_eq!(s.review_queue.items.len(), 4);
        let keys: Vec<&str> = s.review_queue.items.iter().map(|i| i.key.as_str()).collect();
        assert!(keys.contains(&"PROJ-2054"));
        assert!(keys.contains(&"PROJ-2104"));
        assert_eq!(s.review_queue.items[0].key, "PROJ-2054", "cho lau nhat len dau");
    }

    #[test]
    fn review_queue_gom_ca_ready_for_release_vi_no_van_dang_cho_nguoi() {
        let mut cfg = Config::default();
        cfg.review_statuses = vec!["Ready for Test".into(), "In Test".into(),
                                   "Ready for Release".into()];
        let mut raw = fixture_9_ticket_that();
        raw.push(issue(
            "PROJ-2094", "Ready for Release", "done",
            Some(("blake.kim", "Blake Kim - Engineering")),
            "2026-07-18T10:00:00.000+0700", "2026-07-22T16:34:16.000+0700", None, None,
        ));
        let s = build(raw, &sprint_fixture(), &cfg, now_fixture(), DisplayMode::Team);
        assert_eq!(s.review_queue.items.len(), 5);
        // nhung no KHONG duoc tinh vao open_issues
        assert_eq!(s.open_issues.len(), 9);
    }

    #[test]
    fn progress_tach_ready_for_release_khoi_closed() {
        let mut raw = fixture_9_ticket_that();
        for (k, st) in [
            ("PROJ-2094", "Ready for Release"),
            ("PROJ-2103", "Ready for Release"),
            ("PROJ-2064", "Closed"),
        ] {
            raw.push(issue(k, st, "done", Some(("evan.diaz", "Evan Diaz - Engineering")),
                           "2026-07-10T10:00:00.000+0700", "2026-07-22T10:00:00.000+0700", None, None));
        }
        let p = build(raw, &sprint_fixture(), &Config::default(), now_fixture(), DisplayMode::Team).progress;
        assert_eq!(p.total, 12);
        assert_eq!(p.done, 3, "Jira coi ca 3 la done");
        assert_eq!(p.pending_release, 2, "nhung 2 trong so do moi chi cho release");
        assert_eq!(p.closed, 1, "chi 1 ticket that su xong");
        assert_eq!(p.in_progress, 6);
        assert_eq!(p.todo, 3);
    }

    #[test]
    fn dem_nguoc_va_co_sap_het_sprint() {
        let s = snap();
        // 23/07 12:00 -> 24/07 19:50 = 31h50m
        assert_eq!(s.seconds_left.unwrap(), 31 * 3600 + 50 * 60);
        assert!(!s.risks.sprint_ending_soon, "con >24h thi chua bao dong");

        // luc 24/07 10:00 thi con 9h50m -> bao dong, va moi ticket mo deu tinh vao badge
        let late = parse_jira_datetime("2026-07-24T10:00:00.000+0700").unwrap();
        let s2 = build(fixture_9_ticket_that(), &sprint_fixture(), &Config::default(), late, DisplayMode::Team);
        assert!(s2.risks.sprint_ending_soon);
        assert_eq!(s2.risks.count, s2.risks.stale.len() + 9);
    }

    #[test]
    fn age_stats_dung() {
        let s = snap();
        // ages: 1,7,13,20,22,23,24,31,55
        assert_eq!(s.age_stats.max_age, 55);
        assert_eq!(s.age_stats.median_age, 22);
        // idles: 1,1,1,1,1,7,16,16,16
        assert_eq!(s.age_stats.max_idle, 16);
        assert_eq!(s.age_stats.median_idle, 1);
    }

    #[test]
    fn ten_goi_doc_duoc_thay_cho_chu_viet_tat() {
        let users: Vec<String> = ["alex.lee", "casey.park", "finn.reed", "evan.diaz",
                                  "blake.kim", "gale.shaw", "drew.cruz"]
            .iter().map(|s| s.to_string()).collect();
        let m = short_name_map(&users);
        assert_eq!(m["alex.lee"], "Alex");
        assert_eq!(m["casey.park"], "Casey");
        assert_eq!(m["finn.reed"], "Finn");
        assert_eq!(m["evan.diaz"], "Evan");
        assert_eq!(m["blake.kim"], "Blake");
        assert_eq!(m["gale.shaw"], "Gale");
        assert_eq!(m["drew.cruz"], "Drew");
    }

    #[test]
    fn trung_ten_goi_thi_them_chu_cai_ho() {
        // Rat hay gap: hai nguoi trung ten goi.
        let users: Vec<String> = ["alex.lee", "alex.park", "evan.diaz"]
            .iter().map(|s| s.to_string()).collect();
        let m = short_name_map(&users);
        assert_eq!(m["alex.lee"], "Alex L.");
        assert_eq!(m["alex.park"], "Alex P.");
        assert_eq!(m["evan.diaz"], "Evan", "nguoi khong trung thi giu ten ngan");
    }

    #[test]
    fn trung_ca_ho_lan_ten_thi_dung_han_username() {
        // Tha dai con hon chi sai nguoi.
        let users: Vec<String> = ["alex.lee", "alex.lee2"]
            .iter().map(|s| s.to_string()).collect();
        let m = short_name_map(&users);
        assert_eq!(m["alex.lee"], "alex.lee");
        assert_eq!(m["alex.lee2"], "alex.lee2");
    }

    #[test]
    fn nhan_dien_khoa_account_id_cua_cloud() {
        assert!(la_account_id("712020:f58131cb-b67d-43c7"));
        assert!(la_account_id("5b10ac8d82e05b22cc7d4ef5"));
        assert!(!la_account_id("sam.hale"));
        assert!(!la_account_id("tuannguyen"));
    }

    #[test]
    fn cloud_account_id_lay_ten_goi_tu_display_name() {
        // Jira Cloud: khoa la accountId — ten goi PHAI rut tu display name,
        // rut tu khoa la ra chuoi hex vo nghia.
        let pairs = vec![
            ("712020:aa-bb".to_string(), Some("Gale Shaw - Engineering".to_string())),
            ("5b10ac8d82e05b22cc7d4ef5".to_string(), Some("Gale Nguyen".to_string())),
            ("sam.hale".to_string(), None),
        ];
        let m = short_name_map_display(&pairs);
        assert_eq!(m["712020:aa-bb"], "Gale S.", "trung ten goi -> chu cai ho tu display");
        assert_eq!(m["5b10ac8d82e05b22cc7d4ef5"], "Gale N.");
        assert_eq!(m["sam.hale"], "Sam", "khoa DC van di duong username");
    }

    #[test]
    fn cloud_khong_co_display_thi_dung_han_khoa_thay_vi_panic() {
        let pairs = vec![("712020:aa-bb".to_string(), None)];
        let m = short_name_map_display(&pairs);
        assert_eq!(m["712020:aa-bb"], "712020:aa-bb");
    }

    #[test]
    fn issue_mang_ten_goi_va_ticket_chua_giao_ghi_ro() {
        let s = snap_ca_sprint();
        let get = |k: &str| s.issues.iter().find(|i| i.key == k).unwrap().clone();
        assert_eq!(get("PROJ-2038").short_name, "Alex");
        assert_eq!(get("PROJ-1910").short_name, "Gale");
        assert_eq!(get("PROJ-1998").short_name, "chua giao");
        assert_eq!(s.by_assignee[0].short, "Casey");
    }

    #[test]
    fn initials_lay_tu_username() {
        assert_eq!(initials_of(Some("alex.lee"), None), "AL");
        assert_eq!(initials_of(Some("gale.shaw"), None), "GS");
        assert_eq!(initials_of(Some("finn.reed"), None), "FR");
        assert_eq!(initials_of(Some("evan.diaz"), None), "ED");
        // khong co username thi rot ve display name
        assert_eq!(initials_of(None, Some("Evan Diaz - Engineering")), "ED");
        assert_eq!(initials_of(None, None), "?");
    }

    #[test]
    fn url_tro_dung_ticket() {
        let s = snap();
        assert_eq!(
            s.issues.iter().find(|i| i.key == "PROJ-2104").unwrap().url,
            "https://jira.example.com/browse/PROJ-2104"
        );
    }

    #[test]
    fn sprint_rong_khong_panic() {
        let s = build(vec![], &sprint_fixture(), &Config::default(), now_fixture(), DisplayMode::Team);
        assert_eq!(s.progress.total, 0);
        assert_eq!(s.progress.percent, 0);
        assert_eq!(s.risks.count, 0);
        assert_eq!(s.age_stats.max_age, 0);
        assert_eq!(s.points.sprint.denominator, 0);
        assert_eq!(s.points.open.denominator, 0);
        assert!(s.by_assignee.is_empty());
    }

    #[test]
    fn issue_co_timestamp_hong_thi_bo_qua_chu_khong_lam_sap_ca_snapshot() {
        let mut raw = fixture_9_ticket_that();
        raw.push(issue("PROJ-9999", "Open", "new", None, "khong-phai-ngay", "cung-vay", None, None));
        let s = build(raw, &sprint_fixture(), &Config::default(), now_fixture(), DisplayMode::Team);
        assert_eq!(s.issues.len(), 9, "ticket hong bi loai, 9 ticket con lai van tinh dung");
    }

    // ------------------------------------------------- display mode: Only Me

    fn snap_mode(raw: Vec<RawIssue>, me: &str, mode: DisplayMode) -> SprintSnapshot {
        let mut cfg = Config::default();
        cfg.me = me.into();
        build(raw, &sprint_fixture(), &cfg, now_fixture(), mode)
    }

    #[test]
    fn only_me_chi_giu_viec_cua_minh() {
        // AC-D6 + AC-D7. `alex.lee` co 9 ticket trong sprint: 4 dang mo, 5 da dong.
        let s = snap_mode(fixture_ca_sprint(), "alex.lee", DisplayMode::OnlyMe);

        assert_eq!(s.issues.len(), 9);
        assert_eq!(s.open_issues.len(), 4);
        assert!(s
            .issues
            .iter()
            .all(|i| i.assignee.as_deref() == Some("alex.lee")));

        // Moi phep phai sinh phai chay tren tap DA LOC, khong duoc sot cai nao
        // van tinh tren ca sprint — do dung la lop loi da xay ra mot lan roi.
        assert_eq!(s.progress.total, 9);
        assert_eq!(s.progress.done, 5);
        assert_eq!(s.risks.stale.len(), 1, "chi PROJ-2054 dung im 7 ngay");
        assert_eq!(s.risks.stale[0].key, "PROJ-2054");
        assert_eq!(s.points.open.sp_sum, 1.5);
        assert_eq!(s.points.open.sp_filled, 2);
        assert_eq!(s.points.open.denominator, 4);
        assert_eq!(s.points.open.score_sum, 2.0);
        assert_eq!(s.points.open.score_filled, 1);

        assert_eq!(s.by_assignee.len(), 1, "chi con mot nguoi tren bang tai");
        assert_eq!(s.by_assignee[0].name, "alex.lee");
        assert_eq!(s.by_assignee[0].total, 9);
        // Bat bien cu van phai dung sau khi loc
        assert_eq!(
            s.by_assignee.iter().map(|m| m.done).sum::<usize>(),
            s.progress.done
        );
        assert!(s.points.sprint.sp_sum >= s.points.open.sp_sum);
    }

    #[test]
    fn only_me_van_giu_boi_canh_ca_sprint() {
        // AC-D8: doi mode khong duoc lam mat cau "ca sprint 37/46 · 80%".
        let team = snap_mode(fixture_ca_sprint(), "alex.lee", DisplayMode::Team);
        let mine = snap_mode(fixture_ca_sprint(), "alex.lee", DisplayMode::OnlyMe);

        for s in [&team, &mine] {
            assert_eq!(s.sprint_context.total, 46);
            assert_eq!(s.sprint_context.done, 37);
            assert_eq!(s.sprint_context.percent, 80);
        }
        // O Team mode boi canh chinh la tien do dang hien -> UI khong hien hai lan
        assert_eq!(team.progress, team.sprint_context);
        // O Only Me thi hai con so nay khac han nhau
        assert_eq!(mine.progress.total, 9);
        assert_ne!(mine.progress, mine.sprint_context);
    }

    #[test]
    fn only_me_khong_om_ticket_chua_giao() {
        // AC-D10: PROJ-1998 khong giao ai. "Chua giao" khong thuoc ve bat ky ai,
        // neu no roi vao Only Me thi moi nguoi deu thay no va deu tuong la cua minh.
        let s = snap_mode(fixture_ca_sprint(), "alex.lee", DisplayMode::OnlyMe);
        assert!(!s.issues.iter().any(|i| i.key == "PROJ-1998"));
        assert!(s.issues.iter().all(|i| i.assignee.is_some()));
        assert!(s.risks.unassigned.is_empty());
    }

    #[test]
    fn only_me_khong_lam_rong_hang_doi_theo_vai_tro() {
        // AC-D17. Hang doi loc theo VAI TRO (Approvers/QCs) chu khong theo nguoi
        // lam. Ticket cho `alex.lee` duyet lai dang do `drew.cruz` lam — no khong
        // phai viec cua alex.lee, nhung bo no di thi Only Me mat sach hang cho.
        let mut raw = fixture_ca_sprint();
        raw.push(voi_approver(
            issue(
                "PROJ-3001", "Ready for Review", "indeterminate",
                Some(("drew.cruz", "Drew Cruz - Engineering")),
                "2026-07-18T10:00:00.000+0700", "2026-07-21T10:00:00.000+0700", None, None,
            ),
            "alex.lee",
        ));

        let team = snap_mode(raw.clone(), "alex.lee", DisplayMode::Team);
        let mine = snap_mode(raw, "alex.lee", DisplayMode::OnlyMe);

        assert_eq!(mine.review_queue.items.len(), 1);
        assert_eq!(mine.review_queue.items[0].key, "PROJ-3001");
        assert!(
            !mine.issues.iter().any(|i| i.key == "PROJ-3001"),
            "van khong phai viec cua minh nen khong nam trong danh sach chinh"
        );
        // Ca ba hang doi giong het nhau o hai mode
        assert_eq!(team.review_queue.items, mine.review_queue.items);
        assert_eq!(team.test_queue.items, mine.test_queue.items);
        assert_eq!(team.release_queue.items, mine.release_queue.items);
    }

    #[test]
    fn only_me_ma_chua_biet_me_la_ai_thi_ve_team() {
        // AC-D5 o tang tinh toan: khong duoc loc ra rong roi de UI tu doan.
        let s = snap_mode(fixture_ca_sprint(), "", DisplayMode::OnlyMe);
        assert_eq!(s.display_mode, DisplayMode::Team);
        assert_eq!(s.issues.len(), 46);
        assert!(s.viewer.is_none());
    }

    #[test]
    fn viewer_mang_ten_goi_de_doc() {
        // AC-D12: chip tren header can `Alex`, khong phai `alex.lee`.
        let s = snap_mode(fixture_ca_sprint(), "alex.lee", DisplayMode::OnlyMe);
        let v = s.viewer.as_ref().expect("co `me` thi phai co viewer");
        assert_eq!(v.name, "alex.lee");
        assert_eq!(v.short, "Alex");
        assert_eq!(v.display, "Alex Lee", "bo duoi phong ban");
    }

    #[test]
    fn xong_het_viec_khac_han_voi_khong_co_viec_nao() {
        // AC-D15 vs AC-D16: hai man hinh cung rong nhung phai noi hai cau khac
        // nhau. Snapshot phai du du lieu de UI phan biet duoc.
        let xong_het = snap_mode(fixture_ca_sprint(), "casey.park", DisplayMode::OnlyMe);
        assert_eq!(xong_het.issues.len(), 10);
        assert!(xong_het.open_issues.is_empty(), "dong het 10 ticket");
        assert_eq!(xong_het.progress.percent, 100);

        let khong_co = snap_mode(fixture_ca_sprint(), "kai.moss", DisplayMode::OnlyMe);
        assert!(khong_co.issues.is_empty());
        assert_eq!(khong_co.progress.total, 0);
        assert_eq!(khong_co.progress.percent, 0, "0/0 la 0% chu khong panic");
        assert_eq!(
            khong_co.viewer.as_ref().unwrap().short,
            "Kai",
            "khong co ticket nao van phai biet dang xem gium ai"
        );
        assert_eq!(khong_co.sprint_context.total, 46);
    }

    #[test]
    fn mau_bam_theo_nguoi_chu_khong_theo_mode() {
        // Mau duoc cap theo `color_order`. Neu no tinh duoi moc loc thi o Only Me
        // chi con mot ten: ticket cua nguoi khac trong ba hang doi mat cham mau,
        // va mau cua chinh minh nhay sang series dau tien.
        let team = snap_mode(fixture_ca_sprint(), "alex.lee", DisplayMode::Team);
        let mine = snap_mode(fixture_ca_sprint(), "alex.lee", DisplayMode::OnlyMe);

        assert_eq!(team.color_order, mine.color_order, "doi mode khong duoc doi mau");
        assert_eq!(team.color_order.len(), 7, "ca 7 nguoi cua sprint");
        assert_eq!(team.color_order[0], "casey.park", "nhieu ticket nhat len dau");
        assert!(
            !team.color_order.iter().any(|n| n == UNASSIGNED),
            "\"chua giao\" khong duoc cap mau dinh danh"
        );
        // Trai nguoc voi by_assignee, cai VAN phai co lot theo mode
        assert_eq!(mine.by_assignee.len(), 1);
    }

    #[test]
    fn team_mode_khong_doi_gi_so_voi_truoc() {
        // AC-D9 duoi dang bat bien: mac dinh van la goc nhin ca team.
        let s = snap_ca_sprint();
        assert_eq!(s.display_mode, DisplayMode::Team);
        assert_eq!(s.issues.len(), 46);
        assert_eq!(s.progress, s.sprint_context);
        assert_eq!(s.all_digest.len(), s.issues.len());
    }
}
