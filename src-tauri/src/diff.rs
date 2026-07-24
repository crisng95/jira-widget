//! So sanh snapshot N-1 vs N de biet co gi thay doi (AC-22) va bien no thanh
//! notification khong lam phien (AC-23).

use crate::config::NotifyConfig;
use crate::snapshot::SprintSnapshot;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Change {
    StatusChanged {
        key: String,
        summary: String,
        from: String,
        to: String,
    },
    AssigneeChanged {
        key: String,
        summary: String,
        from: Option<String>,
        to: Option<String>,
    },
    Added {
        key: String,
        summary: String,
        status: String,
    },
    Removed {
        key: String,
        summary: String,
    },
}

impl Change {
    /// Hien chi test dung toi, nhung giu lai vi day la accessor hien nhien
    /// cua mot enum public — bo di thi lan sau lai phai viet lai.
    #[allow(dead_code)]
    pub fn key(&self) -> &str {
        match self {
            Change::StatusChanged { key, .. }
            | Change::AssigneeChanged { key, .. }
            | Change::Added { key, .. }
            | Change::Removed { key, .. } => key,
        }
    }

    pub fn enabled_by(&self, cfg: &NotifyConfig) -> bool {
        match self {
            Change::StatusChanged { .. } => cfg.status_changed,
            Change::AssigneeChanged { .. } => cfg.assignee_changed,
            Change::Added { .. } => cfg.added,
            Change::Removed { .. } => cfg.removed,
        }
    }

    pub fn line(&self) -> String {
        match self {
            Change::StatusChanged { key, from, to, .. } => format!("{key}: {from} → {to}"),
            Change::AssigneeChanged { key, from, to, .. } => format!(
                "{key}: {} → {}",
                from.as_deref().unwrap_or("chua giao"),
                to.as_deref().unwrap_or("chua giao")
            ),
            Change::Added { key, status, .. } => format!("{key} vao sprint ({status})"),
            Change::Removed { key, .. } => format!("{key} roi sprint"),
        }
    }
}

/// Doi sprint thi tra ve rong: sang sprint moi ma bao "40 ticket vua bi go"
/// thi vo nghia va rat phien.
///
/// So tren `all_digest` chu khong phai `issues`: `issues` bi loc khi dang xem
/// Only Me, ma thong bao thi phai chay tren CA sprint o moi mode (AC-D18).
/// Ticket bi doi assignee sang nguoi khac roi khoi tap Only Me dung luc no
/// thay doi — neu diff tren tap da loc thi bao "ticket bi go khoi sprint",
/// vua sai vua dung im khi can noi nhat.
pub fn diff(prev: &SprintSnapshot, next: &SprintSnapshot) -> Vec<Change> {
    if prev.sprint_id != next.sprint_id {
        log::info!(
            "sprint doi {} -> {}, bo qua diff lan nay",
            prev.sprint_id,
            next.sprint_id
        );
        return Vec::new();
    }

    let mut out = Vec::new();

    for n in &next.all_digest {
        match prev.all_digest.iter().find(|p| p.key == n.key) {
            None => out.push(Change::Added {
                key: n.key.clone(),
                summary: n.summary.clone(),
                status: n.status.clone(),
            }),
            Some(p) => {
                if p.status != n.status {
                    out.push(Change::StatusChanged {
                        key: n.key.clone(),
                        summary: n.summary.clone(),
                        from: p.status.clone(),
                        to: n.status.clone(),
                    });
                }
                if p.assignee != n.assignee {
                    out.push(Change::AssigneeChanged {
                        key: n.key.clone(),
                        summary: n.summary.clone(),
                        from: p.assignee_display.clone().or_else(|| p.assignee.clone()),
                        to: n.assignee_display.clone().or_else(|| n.assignee.clone()),
                    });
                }
            }
        }
    }

    for p in &prev.all_digest {
        if !next.all_digest.iter().any(|n| n.key == p.key) {
            out.push(Change::Removed {
                key: p.key.clone(),
                summary: p.summary.clone(),
            });
        }
    }

    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub title: String,
    pub body: String,
}

/// Loc theo toggle roi gom nhom khi qua nhieu (AC-23).
pub fn to_notifications(
    changes: &[Change],
    cfg: &NotifyConfig,
    project: &str,
) -> Vec<Notification> {
    let kept: Vec<&Change> = changes.iter().filter(|c| c.enabled_by(cfg)).collect();
    if kept.is_empty() {
        return Vec::new();
    }

    if kept.len() <= cfg.group_threshold {
        return kept
            .iter()
            .map(|c| Notification {
                title: match c {
                    Change::StatusChanged { .. } => format!("{project} · doi trang thai"),
                    Change::AssigneeChanged { .. } => format!("{project} · doi nguoi lam"),
                    Change::Added { .. } => format!("{project} · ticket moi"),
                    Change::Removed { .. } => format!("{project} · ticket roi sprint"),
                },
                body: c.line(),
            })
            .collect();
    }

    // Qua nguong -> 1 notification tong hop, liet ke toi da 4 dong dau.
    let mut body: Vec<String> = kept.iter().take(4).map(|c| c.line()).collect();
    if kept.len() > 4 {
        body.push(format!("... va {} thay doi khac", kept.len() - 4));
    }
    vec![Notification {
        title: format!("{project} · {} thay doi", kept.len()),
        body: body.join("\n"),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DisplayMode};
    use crate::jira::{parse_jira_datetime, RawFields, RawIssue, RawNamed, RawStatus,
                      RawStatusCategory, RawUser, SprintMeta};
    use crate::snapshot::build;

    fn sprint(id: u64) -> SprintMeta {
        SprintMeta {
            id,
            name: format!("Sprint {id}"),
            board_id: 1000,
            start: None,
            end: parse_jira_datetime("2026-07-24T19:50:00.000+0700").ok(),
        }
    }

    fn mk(key: &str, status: &str, cat: &str, assignee: Option<&str>) -> RawIssue {
        RawIssue {
            key: key.into(),
            fields: RawFields {
                summary: format!("{key} tom tat"),
                status: RawStatus {
                    name: status.into(),
                    category: RawStatusCategory { key: cat.into() },
                },
                assignee: assignee.map(|n| RawUser {
                    name: Some(n.into()),
                    display_name: Some(n.to_uppercase()),
                }),
                created: "2026-07-10T10:00:00.000+0700".into(),
                updated: "2026-07-22T10:00:00.000+0700".into(),
                issuetype: Some(RawNamed { name: "Task".into() }),
                priority: Some(RawNamed { name: "Medium".into() }),
                story_point: None,
                app_task_score: None,
                approvers: None,
                qcs: None,
            },
        }
    }

    fn snap(id: u64, raw: Vec<RawIssue>) -> SprintSnapshot {
        snap_mode(id, raw, DisplayMode::Team, "")
    }

    fn snap_mode(id: u64, raw: Vec<RawIssue>, mode: DisplayMode, me: &str) -> SprintSnapshot {
        let mut cfg = Config::default();
        cfg.me = me.into();
        build(
            raw,
            &sprint(id),
            &cfg,
            parse_jira_datetime("2026-07-23T12:00:00.000+0700").unwrap(),
            mode,
        )
    }

    #[test]
    fn khong_doi_gi_thi_khong_co_change() {
        let a = snap(9302, vec![mk("PROJ-1", "In Progress", "indeterminate", Some("evan.diaz"))]);
        let b = snap(9302, vec![mk("PROJ-1", "In Progress", "indeterminate", Some("evan.diaz"))]);
        assert!(diff(&a, &b).is_empty());
    }

    #[test]
    fn bat_duoc_doi_status() {
        let a = snap(9302, vec![mk("PROJ-1", "In Progress", "indeterminate", Some("evan.diaz"))]);
        let b = snap(9302, vec![mk("PROJ-1", "Ready for Test", "indeterminate", Some("evan.diaz"))]);
        let d = diff(&a, &b);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].line(), "PROJ-1: In Progress → Ready for Test");
    }

    #[test]
    fn bat_duoc_doi_assignee_va_ca_truong_hop_chua_giao() {
        let a = snap(9302, vec![mk("PROJ-1", "Open", "new", None)]);
        let b = snap(9302, vec![mk("PROJ-1", "Open", "new", Some("evan.diaz"))]);
        let d = diff(&a, &b);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].line(), "PROJ-1: chua giao → EVAN.DIAZ");
    }

    #[test]
    fn bat_duoc_them_va_bot_ticket() {
        let a = snap(9302, vec![mk("PROJ-1", "Open", "new", None)]);
        let b = snap(9302, vec![mk("PROJ-2", "Open", "new", None)]);
        let d = diff(&a, &b);
        assert_eq!(d.len(), 2);
        assert!(d.iter().any(|c| matches!(c, Change::Added { .. }) && c.key() == "PROJ-2"));
        assert!(d.iter().any(|c| matches!(c, Change::Removed { .. }) && c.key() == "PROJ-1"));
    }

    #[test]
    fn doi_sprint_thi_im_lang() {
        let a = snap(9302, vec![mk("PROJ-1", "Open", "new", None)]);
        let b = snap(9400, vec![mk("PROJ-50", "Open", "new", None)]);
        assert!(diff(&a, &b).is_empty(), "sang sprint moi khong duoc spam");
    }

    #[test]
    fn only_me_van_bao_thay_doi_cua_ca_sprint() {
        // AC-D18. Hai ticket, khong cai nao cua `evan.diaz` — o Only Me thi ca
        // hai deu bien khoi man hinh, nhung thong bao van phai bat duoc.
        let a = snap_mode(
            9302,
            vec![
                mk("PROJ-1", "In Progress", "indeterminate", Some("dana.roy")),
                mk("PROJ-2", "Open", "new", Some("dana.roy")),
            ],
            DisplayMode::OnlyMe,
            "evan.diaz",
        );
        let b = snap_mode(
            9302,
            vec![
                mk("PROJ-1", "Ready for Test", "indeterminate", Some("dana.roy")),
                mk("PROJ-2", "Open", "new", Some("dana.roy")),
            ],
            DisplayMode::OnlyMe,
            "evan.diaz",
        );
        assert!(a.issues.is_empty(), "man hinh Only Me dung la rong");
        let d = diff(&a, &b);
        assert_eq!(d.len(), 1, "nhung diff van phai thay doi status cua PROJ-1");
        assert_eq!(d[0].line(), "PROJ-1: In Progress → Ready for Test");
    }

    #[test]
    fn ticket_bi_chuyen_cho_nguoi_khac_khong_bi_bao_nham_la_roi_sprint() {
        // Ca de nhat: `evan.diaz` mat ticket vi bi doi assignee. Neu diff chay
        // tren tap da loc thi day se thanh "PROJ-1 roi sprint" — sai su that.
        let a = snap_mode(
            9302,
            vec![mk("PROJ-1", "Open", "new", Some("evan.diaz"))],
            DisplayMode::OnlyMe,
            "evan.diaz",
        );
        let b = snap_mode(
            9302,
            vec![mk("PROJ-1", "Open", "new", Some("dana.roy"))],
            DisplayMode::OnlyMe,
            "evan.diaz",
        );
        assert_eq!(a.issues.len(), 1);
        assert!(b.issues.is_empty(), "ticket da roi khoi man hinh Only Me");

        let d = diff(&a, &b);
        assert_eq!(d.len(), 1);
        assert!(
            matches!(d[0], Change::AssigneeChanged { .. }),
            "phai la doi nguoi lam, khong phai roi sprint: {:?}",
            d[0]
        );
        assert_eq!(d[0].line(), "PROJ-1: EVAN.DIAZ → DANA.ROY");
    }

    #[test]
    fn duoi_nguong_thi_bao_rieng_tung_cai() {
        let cfg = NotifyConfig::default(); // group_threshold = 3
        let ch = vec![
            Change::StatusChanged { key: "A".into(), summary: "".into(), from: "x".into(), to: "y".into() },
            Change::Added { key: "B".into(), summary: "".into(), status: "Open".into() },
        ];
        assert_eq!(to_notifications(&ch, &cfg, "PROJ").len(), 2);
    }

    #[test]
    fn vuot_nguong_thi_gom_thanh_mot() {
        let cfg = NotifyConfig::default();
        let ch: Vec<Change> = (0..6)
            .map(|i| Change::StatusChanged {
                key: format!("PROJ-{i}"),
                summary: "".into(),
                from: "In Progress".into(),
                to: "Ready for Test".into(),
            })
            .collect();
        let n = to_notifications(&ch, &cfg, "PROJ");
        assert_eq!(n.len(), 1, "6 thay doi -> 1 notification thay vi 6");
        assert_eq!(n[0].title, "PROJ · 6 thay doi");
        assert!(n[0].body.contains("va 2 thay doi khac"));
    }

    #[test]
    fn tat_toggle_thi_khong_bao() {
        let mut cfg = NotifyConfig::default();
        cfg.status_changed = false;
        let ch = vec![Change::StatusChanged {
            key: "A".into(), summary: "".into(), from: "x".into(), to: "y".into(),
        }];
        assert!(to_notifications(&ch, &cfg, "PROJ").is_empty());
    }
}
