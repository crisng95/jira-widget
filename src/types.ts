// Ban sao cua struct ben Rust (snapshot.rs / poller.rs). Rust serialize camelCase.
// Webview KHONG bao gio tu goi Jira — moi con so o day deu do Rust tinh san.

export interface Issue {
  key: string;
  summary: string;
  status: string;
  /** "new" | "indeterminate" | "done" */
  statusCategory: string;
  assignee: string | null;
  assigneeDisplay: string | null;
  initials: string;
  /** Ten goi de doc, vd `Tuan` — thay cho chu viet tat */
  shortName: string;
  issueType: string;
  priority: string;
  created: string;
  updated: string;
  ageDays: number;
  idleDays: number;
  storyPoint: number | null;
  appTaskScore: number | null;
  url: string;
  isOpen: boolean;
  isPendingRelease: boolean;
  isStale: boolean;
  isOld: boolean;
  /** `me` nam trong Approvers (cf_10200) — vai tro DUYET */
  isApproverMe: boolean;
  /** `me` nam trong QCs (cf_10201) — vai tro TEST */
  isQcMe: boolean;
}

export interface Progress {
  total: number;
  /** Jira coi la Done — gom ca Ready for Release */
  done: number;
  closed: number;
  /** Jira tinh Done nhung thuc te moi cho release */
  pendingRelease: number;
  inProgress: number;
  todo: number;
  percent: number;
}

export interface Risks {
  stale: Issue[];
  endingSoon: Issue[];
  unassigned: Issue[];
  sprintEndingSoon: boolean;
  count: number;
}

export interface StatusCount {
  status: string;
  count: number;
}

/** Tinh tren TOAN BO ticket sprint — khong chi ticket dang mo. */
export interface MemberLoad {
  name: string;
  display: string;
  initials: string;
  /** Ten goi de doc, vd `Tuan` */
  short: string;
  total: number;
  done: number;
  open: number;
  inProgress: number;
  todo: number;
  donePercent: number;
  spSum: number;
  scoreSum: number;
  byStatus: StatusCount[];
  isUnassigned: boolean;
  /** Dong nay la chinh nguoi dung (`config.me`) */
  isMe: boolean;
}

/** Luon di kem mau so — du lieu that rat thua nen So tran trui se gay hieu sai. */
export interface PointScope {
  spSum: number;
  spFilled: number;
  scoreSum: number;
  scoreFilled: number;
  denominator: number;
}

export interface PointTotals {
  /** Ca sprint */
  sprint: PointScope;
  /** Chi ticket chua xong — tap con cua `sprint` */
  open: PointScope;
}

/** Mot hang doi cho viec, mang theo pham vi va co an/hien. */
export interface Queue {
  items: Issue[];
  /** "mine" | "all" */
  scope: string;
  visible: boolean;
}

export interface AgeStats {
  medianAge: number;
  maxAge: number;
  medianIdle: number;
  maxIdle: number;
}

/** Rust serialize enum nay ra snake_case. */
export type DisplayMode = "team" | "only_me";

/** Nguoi dang xem panel — chi co khi da dat `me` trong Cai dat. */
export interface Viewer {
  name: string;
  /** Ten goi de doc, vd `Tuan` */
  short: string;
  display: string;
}

export interface SprintSnapshot {
  fetchedAt: string;
  sprintId: number;
  sprintName: string;
  sprintEnd: string | null;
  secondsLeft: number | null;
  /** Da loc theo mode — o `only_me` chi con ticket cua minh */
  issues: Issue[];
  openIssues: Issue[];
  progress: Progress;
  /** Tien do CA sprint, luon tinh truoc khi loc. O `team` no trung `progress`. */
  sprintContext: Progress;
  displayMode: DisplayMode;
  viewer: Viewer | null;
  /** Username xếp theo số ticket giảm dần, tính trên CẢ sprint. */
  colorOrder: string[];
  risks: Risks;
  byAssignee: MemberLoad[];
  /** Cho test — loc theo QCs */
  testQueue: Queue;
  /** Cho duyet — loc theo Approvers */
  reviewQueue: Queue;
  /** Cho release — khong loc theo nguoi */
  releaseQueue: Queue;
  points: PointTotals;
  ageStats: AgeStats;
}

export interface PanelState {
  snapshot: SprintSnapshot | null;
  ok: boolean;
  /** "auth" | "network" | "api" | "parse" */
  errorKind: string | null;
  errorMessage: string | null;
  lastSuccess: string | null;
  consecutiveFailures: number;
  noActiveSprint: boolean;
  pollIntervalSecs: number;
  staleDays: number;
  /** "vi" | "en" — de panel chon dung bo chuoi ngay tu lan render dau */
  language: string;
  /** Chua cai dat xong — panel hien man chao mung thay vi loi ket noi */
  onboarding: boolean;
}

/** Thu tu co dinh — mau bam theo NGUOI, khong bam theo thu hang.
 *  Loc bot member khong duoc lam doi mau nhung nguoi con lai. */
export const SERIES_VARS = [
  "var(--series-1)",
  "var(--series-2)",
  "var(--series-3)",
  "var(--series-4)",
  "var(--series-5)",
  "var(--series-6)",
] as const;

/** Nhung member duoc ve rieng trong donut: 6 nguoi nhieu task nhat.
 *  `members` da duoc Rust sap giam dan theo total, "chua giao" o cuoi. */
export function shownMembers(members: MemberLoad[]): MemberLoad[] {
  return members.filter((m) => !m.isUnassigned).slice(0, SERIES_VARS.length);
}

/** Gan mau cho DUNG nhom se duoc hien, theo alphabet trong nhom do.
 *
 *  Ban truoc gan mau cho 6 nguoi dau alphabet cua CA danh sach, trong khi
 *  donut lai ve 6 nguoi nhieu task nhat. Hai tap trung nhau khi chi co 6 member
 *  nen bug ngu yen; len 8 member thi nguoi lech tap se render ra xam.
 *
 *  Nhan `colorOrder` — do Rust tinh tren CA sprint — chu khong nhan
 *  `byAssignee`: o Only Me `byAssignee` chi con mot dong, nen ticket cua nguoi
 *  khac trong ba hang doi se mat cham mau va mau cua chinh minh nhay sang
 *  series dau. Mau phai bam theo NGUOI, khong bam theo mode dang xem.
 *
 *  Danh doi: ai do ra/vao nhom top-6 thi mau co the xe dich. Chap nhan duoc —
 *  thu hang chi quyet dinh AI DUOC HIEN, con alphabet quyet dinh MAU NAO,
 *  nen day khong phai gan-mau-theo-thu-hang. */
export function colorMap(colorOrder: string[]): Map<string, string> {
  const names = colorOrder
    .slice(0, SERIES_VARS.length)
    .sort((a, b) => a.localeCompare(b));
  const map = new Map<string, string>();
  names.forEach((n, i) => map.set(n, SERIES_VARS[i]));
  return map;
}

export const OTHER_COLOR = "var(--baseline)";

/** Khoa kieu accountId cua Jira Cloud (GDPR bo username) — ban TS cua
 *  `la_account_id` ben Rust. Khoa nay chi de tra mau/so khop, KHONG duoc
 *  hien tho ra giao dien. */
export function laAccountId(u: string): boolean {
  return u.includes(":") || (u.length >= 20 && /^[0-9a-fA-F]+$/.test(u));
}

/** Nhan hien thi cho mot member: username doc duoc (DC) hay display name
 *  (Cloud — `name` la accountId tho). Mau va key Svelte van dung `m.name`. */
export function memberLabel(m: MemberLoad): string {
  return laAccountId(m.name) ? m.display : m.name;
}

/** So thap phan gon: 6.5 -> "6.5", 7 -> "7" */
export function fmtNum(n: number): string {
  return Number.isInteger(n) ? String(n) : n.toFixed(1);
}
