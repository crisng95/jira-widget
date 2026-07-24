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

/// Rut username tu mot field kieu mang, chiu duoc ca `["ten"]` lan
/// `[{"name": "ten", ...}]`.
pub fn usernames_from(v: &Option<serde_json::Value>) -> Vec<String> {
    let Some(serde_json::Value::Array(arr)) = v else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|x| match x {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Object(o) => {
                o.get("name").and_then(|n| n.as_str()).map(String::from)
            }
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
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,
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

pub struct JiraClient {
    http: reqwest::Client,
    base: String,
    token: String,
    sprint_cache: Mutex<Option<(SprintMeta, Instant)>>,
}

impl JiraClient {
    pub fn new(base_url: &str, token: String) -> Result<Self> {
        // Cert cua jira.example.com do GlobalSign cap va con han
        // -> GIU verification nghiem ngat, khong dung danger_accept_invalid_certs.
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .user_agent("jira-widget/0.1")
            .build()?;
        Ok(Self {
            http,
            base: base_url.trim_end_matches('/').to_string(),
            token,
            sprint_cache: Mutex::new(None),
        })
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, JiraError> {
        let url = format!("{}{}", self.base, path);
        log::debug!("GET {url}");

        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.token)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
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
            return Err(JiraError::Auth(format!("HTTP {}", status.as_u16())));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(JiraError::Api {
                status: status.as_u16(),
                body,
            });
        }

        resp.json::<T>()
            .await
            .map_err(|e| JiraError::Parse(e.to_string()))
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
