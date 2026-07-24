# App Store Connect — bộ thông tin submit

App record trên ASC: bundle id **`com.isemi.jiraw`** (đã tạo).
Repo đã được cập nhật khớp: `src-tauri/tauri.conf.json` → `"identifier": "com.isemi.jiraw"`,
và `src-tauri/Info.plist` đã khai `ITSAppUsesNonExemptEncryption=false` + category Productivity.

Tài liệu này soạn sẵn **mọi field phải điền trên ASC** (mục 1–7) và **checklist kỹ thuật
để bản build qua được review** (mục 8). Chỗ nào cần anh quyết có đánh dấu ⚠️.

Toàn bộ metadata đã đóng gói máy-đọc-được ở **`fastlane/metadata/`** — đẩy lên ASC bằng:

```bash
ASC_ISSUER_ID=<issuer-uuid> fastlane mac push_metadata
```

Tên app đã chốt: **Master Jira** (đã áp dụng toàn repo + metadata).

---

## 1. App Information

| Field | Giá trị đề xuất |
|---|---|
| Name (≤30 ký tự) | **`Master Jira`** (11) — đã chốt; xem ghi chú trademark bên dưới |
| Subtitle (≤30) | EN: `Active sprint on your desktop` (29) |
| Bundle ID | `com.isemi.jiraw` |
| SKU | `jiraw-001` (nội bộ, không public, không đổi được sau này) |
| Primary language | **English (U.S.)** — thêm localization **Vietnamese** (mục 5) |
| Primary category | **Productivity** |
| Secondary category | **Developer Tools** |
| Content rights | Chọn **"No, it does not contain, show, or access third-party content"** — app chỉ hiển thị dữ liệu Jira của chính người dùng, không phân phối nội dung bên thứ ba |
| Age rating | Trả lời **None/No** toàn bộ bảng câu hỏi → **4+** |
| License Agreement | Dùng Apple standard EULA (không cần custom) |

### ⚠️ Ghi chú tên app (trademark — Guideline 5.2.1)

Đã chốt **Master Jira** và áp dụng toàn repo (productName, title các cửa sổ, README,
fastlane metadata). Mức rủi ro, nói thẳng: tên không *bắt đầu* bằng "Jira" (tránh được
lỗi nặng nhất), nhưng cũng không theo mẫu *"Tên riêng for Jira"* mà guideline thương hiệu
bên-thứ-ba của Atlassian khuyến nghị. Nếu App Review vướng 5.2.1 hoặc Atlassian khiếu nại,
phương án lùi là **`Master for Jira`** (15 ký tự) — chỉ sửa metadata, không cần build lại
binary. Description hai ngôn ngữ đã kèm dòng disclaimer "không liên kết với Atlassian"
để giảm rủi ro này.

Bundle id `com.isemi.jiraw` không sao — bundle id không hiển thị và không dính trademark.

---

## 2. Pricing & Availability

| Field | Giá trị |
|---|---|
| Price | **Free (0đ)** |
| Availability | Tất cả territories (hoặc thu hẹp tuỳ anh) |
| Pre-order | Không |

⚠️ Nếu app chủ đích chỉ cho team/khách nội bộ: sau khi được **approve**, có thể nộp form
xin **Unlisted App Distribution** — app chỉ cài được qua link trực tiếp, không hiện trên
search/charts. Phù hợp tool nội bộ mà vẫn hưởng notarize + auto-update qua App Store.

---

## 3. App Privacy

| Câu hỏi ASC | Trả lời |
|---|---|
| Privacy Policy URL | `https://github.com/crisng95/jira-widget/blob/main/docs/privacy-policy.md` — file đã soạn sẵn trong repo, **nhớ push lên GitHub trước khi submit** (hoặc thay bằng trang trên domain isemi nếu có) |
| Does this app collect data? | **No — Data Not Collected** |

Căn cứ (đúng với code hiện tại, đã kiểm):
- Không analytics/crash-reporting/ads SDK; binary chỉ gọi tới đúng host Jira user cấu hình.
- PAT nằm trong macOS Keychain, chỉ gửi tới Jira của user qua HTTPS.
- Config + log ghi cục bộ (`~/Library/Application Support/jira-widget/`, `~/Library/Logs/`).
- "Collect" theo định nghĩa của Apple = dữ liệu rời khỏi thiết bị về phía developer/bên thứ ba
  → app này không có luồng nào như vậy.

---

## 4. Bản listing tiếng Anh (primary — English U.S.)

### Promotional Text (≤170 — sửa được không cần review lại)

```
See your active sprint at a glance — progress, risks, handover queues and team load — right on your desktop. Read-only and private: your data never leaves your Mac.
```
(164 ký tự)

### Description (≤4000)

```
Your active sprint, always in sight.

Master Jira pins a compact, glanceable panel to your desktop showing the live state of your team's active sprint — no browser tab, no context switching. It refreshes every 60 seconds and taps you on the shoulder only when something actually changes.

BUILT FOR TEAM LEADS
• Sprint progress at a glance — done vs. total, story points with explicit denominators, and "pending release" split out from truly done
• Risk alerts — stale tickets nobody has touched, sprint ending soon, long-lived tickets
• Handover queues — Ready for Review, waiting for your approval, waiting for release
• Team load — per-member progress bars and a workload donut for the whole sprint
• Native notifications — status changes, reassignments, tickets added to or pulled from the sprint; bursts are grouped into a single summary

ONE PANEL, TWO VIEWS
Switch between "whole team" and "only my tickets" from the menu bar. The panel re-filters instantly from the latest data — numbers, alerts and the menu-bar badge all follow.

STAYS OUT OF YOUR WAY
• Desktop mode: the panel sits with your desktop wallpaper, under every window — visible when you glance at the desktop, never covering your work. Or switch to floating mode to keep it always on top.
• Menu-bar app with start-at-login, compact mode and one-click refresh.
• Click any ticket to open it in your browser.

PRIVATE BY DESIGN
• Read-only: the app never writes anything to Jira
• Your Personal Access Token is stored in the macOS Keychain and never leaves your Mac
• Talks only to the Jira server you configure — no analytics, no tracking, no third-party servers

REQUIREMENTS
• Jira Server or Data Center 8.14 or later (Personal Access Tokens). Jira Cloud is not supported yet.
• A Jira account with permission to view the board you follow.
• English and Vietnamese interface.

Jira is a registered trademark of Atlassian Pty Ltd. This app is an independent product and is not affiliated with, endorsed by, or sponsored by Atlassian.
```
(2.007 ký tự — còn dư nhiều)

### Keywords (≤100, phân cách bằng dấu phẩy, không space sau phẩy)

```
scrum,agile,standup,kanban,board,team,dashboard,tracker,widget,menubar,backlog,issue
```
(84 ký tự — không lặp "jira"/"sprint" vì name "Master Jira" + subtitle đã được index sẵn)

### URLs + Copyright

| Field | Giá trị |
|---|---|
| Support URL | `https://github.com/crisng95/jira-widget/issues` |
| Marketing URL (optional) | `https://github.com/crisng95/jira-widget` |
| Copyright | `2026 ISEMI COMPANY LIMITED` |

### What's New

Bản 1.0 **không có** field này — chỉ xuất hiện từ bản update thứ hai. Mẫu cho lần sau:

```
• Bug fixes and performance improvements.
```

---

## 5. Localization tiếng Việt (thêm ở tab Vietnamese)

| Field | Giá trị |
|---|---|
| Subtitle (≤30) | `Theo dõi sprint trên desktop` (28) |
| Keywords | `scrum,agile,kanban,nhóm,bảng công việc,tiến độ,quản lý dự án,standup,backlog` (76) — bỏ "jira"/"sprint" vì name + subtitle đã chứa |
| Support URL | như bản EN |

### Promotional Text (≤170)

```
Sprint đang chạy hiện ngay trên desktop: tiến độ, cảnh báo rủi ro, hàng đợi bàn giao, tải của từng người. Chỉ đọc và riêng tư — dữ liệu không rời khỏi máy bạn.
```
(159 ký tự)

### Description

```
Sprint đang chạy, lúc nào cũng trong tầm mắt.

Master Jira ghim một panel gọn lên desktop, hiển thị trạng thái trực tiếp của active sprint — không cần mở tab trình duyệt, không phải đổi ngữ cảnh. Panel tự làm mới mỗi 60 giây và chỉ báo khi có thứ thực sự thay đổi.

SINH RA CHO TEAM LEAD
• Tiến độ sprint trong một cái liếc — done/tổng, story point luôn kèm mẫu số, và "chờ release" tách riêng khỏi done thật
• Cảnh báo rủi ro — ticket đứng im nhiều ngày, sprint sắp hết, ticket sống quá lâu
• Hàng đợi bàn giao — chờ review, chờ chính bạn duyệt, chờ release
• Tải của team — thanh tiến độ từng thành viên và donut phân bổ khối lượng cả sprint
• Thông báo native — đổi status, đổi assignee, ticket vào/ra sprint; nhiều thay đổi cùng lúc được gom thành một thông báo tổng hợp

MỘT PANEL, HAI GÓC NHÌN
Chuyển giữa "cả team" và "chỉ việc của tôi" ngay từ menu bar. Panel lọc lại tức thì trên dữ liệu mới nhất — mọi con số, cảnh báo và badge trên menu bar đổi theo.

KHÔNG BAO GIỜ CHOÁN CHỖ
• Chế độ desktop: panel dán vào nền desktop, nằm dưới mọi cửa sổ — liếc desktop là thấy, không bao giờ che việc đang làm. Hoặc chuyển sang chế độ nổi để luôn hiện trên cùng.
• App menu bar: khởi động cùng máy, chế độ thu gọn, refresh một chạm.
• Bấm vào ticket là mở thẳng trong trình duyệt.

RIÊNG TƯ TỪ THIẾT KẾ
• Chỉ đọc: app không bao giờ ghi gì lên Jira
• Personal Access Token nằm trong Keychain của macOS, không bao giờ rời khỏi máy
• Chỉ kết nối tới đúng máy chủ Jira bạn cấu hình — không analytics, không tracking, không máy chủ bên thứ ba

YÊU CẦU
• Jira Server hoặc Data Center 8.14 trở lên (Personal Access Token). Chưa hỗ trợ Jira Cloud.
• Tài khoản Jira có quyền xem board bạn theo dõi.
• Giao diện tiếng Việt và tiếng Anh.

Jira là nhãn hiệu đã đăng ký của Atlassian Pty Ltd. Đây là sản phẩm độc lập, không liên kết, không được bảo trợ hay chứng thực bởi Atlassian.
```

---

## 6. Screenshots (bắt buộc ≥1, tối đa 10)

**Kích thước Mac hợp lệ (16:10):** 1280×800 · 1440×900 · 2560×1600 · **2880×1800** (khuyên dùng).
PNG hoặc JPEG, không alpha. `docs/panel.png` hiện tại (406×976) **không dùng được**.

Cách chụp chuẩn 2880×1800 trên màn Retina 2x — chụp vùng 1440×900 point:

```bash
screencapture -R "0,25,1440,900" -x ~/Desktop/jiraw-1.png
sips -g pixelWidth -g pixelHeight ~/Desktop/jiraw-1.png   # phải ra 2880×1800
```

Bộ 5 ảnh đề xuất (dọn desktop, wallpaper gọn, dữ liệu demo không lộ tên người thật/URL nội bộ):

1. Panel chế độ team đầy đủ trên desktop — hero shot
2. Cận cảnh khối cảnh báo rủi ro + ba hàng đợi bàn giao
3. Chế độ "Chỉ việc của tôi" (có chip `● Tên · chỉ việc của tôi`)
4. Cửa sổ Cài đặt — ô kiểm tra kết nối + board picker
5. Menu tray đang mở + một notification thật ở góc màn hình

UI đã có i18n vi/en theo ngôn ngữ hệ điều hành — chụp bộ EN cho locale en-US, bộ VI cho
locale vi (ASC cho phép screenshots khác nhau theo từng locale).

---

## 7. App Review Information

| Field | Giá trị |
|---|---|
| Contact first/last name | ⚠️ điền tên anh |
| Phone | ⚠️ số liên lạc được (kèm mã +84) |
| Email | aabooksapp@gmail.com |
| Sign-in required | **Yes** → bắt buộc cung cấp cách test (xem cảnh báo dưới) |

### ⚠️ Vấn đề lớn nhất của lần submit này: reviewer phải test được app (Guideline 2.1)

App đòi Jira Server/DC + PAT. Jira của mình nằm sau VPN → **reviewer không kết nối được
→ gần như chắc chắn bị reject "we were unable to review your app"**. Ba đường ra, xếp theo
độ thực dụng:

1. **Thêm Demo mode (khuyên làm — nhỏ):** nút "Dùng dữ liệu mẫu" trong Settings, nạp
   snapshot từ bộ fixture 9 ticket đã có sẵn trong test của `snapshot.rs`/`diff.rs`,
   panel hiện banner "DEMO". Reviewer không cần server nào cả. Ghi rõ trong Review Notes.
2. Dựng một Jira DC trial public tạm (đắt công, phải sống suốt thời gian review).
3. Chỉ nộp video demo qua attachment — App Review vẫn thường đòi tự bấm được app, rủi ro cao.

### Review Notes (paste nguyên khối, đã kèm chỗ trống demo)

```
WHAT THIS APP IS
A read-only desktop panel for macOS showing the state of the active sprint on one Agile board of a Jira Server / Data Center instance. Aimed at team leads. It polls the Jira REST API (agile/1.0, api/2) over HTTPS every 60 seconds and renders a summary. It never creates, edits, or transitions anything in Jira.

HOW TO TEST
1. On first launch an onboarding window opens. The UI is localized in English and Vietnamese and follows the macOS system language.
2. Enter the Jira Server/DC base URL and a Personal Access Token, test the connection, pick a board, and save. The panel then appears pinned to the desktop layer (glance at the desktop or press F11 to see it).
3. The menu-bar icon has Show/Hide, Collapse/Expand, Only-my-work, Refresh, Move panel, Settings, Start with macOS, and Quit.

DEMO ACCESS
[FILL IN — public demo Jira URL + Personal Access Token reachable from the reviewer's network]
[OR if demo mode ships in this build: choose "Use sample data" in the onboarding window — no server or account needed.]

NOTES
• All network traffic goes exclusively to the Jira host the user configures. No developer server, no analytics, no data collection of any kind. The Personal Access Token is stored in the macOS Keychain.
• Works with Jira Server / Data Center (PAT auth). Jira Cloud is intentionally not supported yet, as stated in the description.
• This is an independent companion app for Atlassian Jira; the description carries the standard trademark disclaimer.
```

Có thể đính kèm thêm **video quay màn hình 1–2 phút** (attachment trong App Review
Information) quay cảnh nhập token → panel chạy → các chế độ. Rất nên làm.

---

## 8. Checklist kỹ thuật để bản build MAS qua được review

Trạng thái hiện tại: app đang build kiểu ad-hoc sign cho .dmg GitHub. Bản nộp Mac App
Store là một cấu hình build **khác**, còn các việc sau:

| # | Việc | Trạng thái |
|---|---|---|
| 1 | Bundle id `com.isemi.jiraw` trong `tauri.conf.json` | ✅ đã đổi (commit này) |
| 2 | `Info.plist`: `ITSAppUsesNonExemptEncryption=false` + `LSApplicationCategoryType` | ✅ đã thêm (commit này) |
| 3 | **Bỏ private API** cho bản MAS | ❌ **blocker.** `macos-private-api` trong `src-tauri/Cargo.toml` + `macOSPrivateApi: true` trong `tauri.conf.json` (dùng cho cửa sổ trong suốt) là private API — Guideline 2.5.1, docs Tauri nói thẳng bật nó là App Store từ chối. Bản MAS phải build không có nó → panel chuyển nền đặc (mất nhìn-xuyên, giữ bo góc bằng CSS trong cửa sổ thường). |
| 4 | **App Sandbox** | ❌ blocker. Tạo `entitlements.mas.plist` (`com.apple.security.app-sandbox` + `com.apple.security.network.client`), khai vào `bundle.macOS.entitlements`, kèm **provisioning profile Mac App Store** cho `com.isemi.jiraw`. |
| 5 | **Keychain code không hợp sandbox** | ❌ blocker. `config.rs` đang gọi CLI `security` (3 chỗ: dòng ~261/284/356) — trong sandbox process con bị kế thừa sandbox và không đụng được login keychain. Đổi sang crate `security-framework`/`keyring` (Security.framework, hợp sandbox). Token cũ không migrate — user nhập lại một lần. |
| 6 | **Autostart không hợp sandbox** | ❌ `main.rs:507` dùng `MacosLauncher::LaunchAgent` (ghi `~/Library/LaunchAgents` — sandbox cấm). Bản MAS: chuyển `SMAppService` (macOS 13+) hoặc ẩn mục "Khởi động cùng máy". |
| 7 | Ký & đóng gói | ❌ Cert **Apple Distribution** + **Mac Installer Distribution** trong ASC → ký .app với entitlements + profile → `xcrun productbuild --sign` ra `.pkg` → upload bằng **Transporter**. Làm theo guide chính thức Tauri v2 "App Store distribution". `signingIdentity: "-"` hiện tại chỉ dành cho bản .dmg ngoài store. |
| 8 | Kiến trúc | Hiện arm64-only — MAS chấp nhận (máy Intel không cài được). Muốn phủ Intel: build `--target universal-apple-darwin`. |
| 9 | Version | Khuyên nâng `version` trong `tauri.conf.json` → **`1.0.0`** cho bản đầu lên store (ASC đối chiếu CFBundleShortVersionString). |
| 10 | Demo mode cho App Review | ❌ xem mục 7 — thực tế là bắt buộc vì Jira sau VPN. |
| 11 | minimumSystemVersion | Đang `11.0` — nếu chuyển autostart sang SMAppService thì nâng lên `13.0`, đồng thời ASC hiện đúng yêu cầu hệ điều hành. |

### Lối ít công hơn nếu mục tiêu chỉ là "teammate cài không bị chặn"

Đã có Apple Developer account rồi thì **Developer ID + notarize** bản .dmg hiện tại là đủ
để double-click chạy ngay, **không cần** sandbox, không cần bỏ private API, không cần demo
account, không qua App Review. Mac App Store chỉ đáng khi muốn phân phối public/unlisted
qua store. Hai đường không loại trừ nhau — có thể notarize .dmg ngay bây giờ, làm bản MAS
sau.

---

## 9. Thứ tự thao tác đề xuất

1. ~~Chốt tên app~~ → đã chốt **Master Jira**, repo + metadata đổi xong.
2. Push repo lên GitHub để **privacy policy URL sống**.
3. Đẩy metadata lên ASC (điền mục 1, 4, 5, privacy URL + review notes tự động):

   ```bash
   ASC_ISSUER_ID=<issuer-uuid> fastlane mac push_metadata
   ```

   Issuer ID lấy ở **ASC → Users and Access → Integrations → App Store Connect API**.
   Key mặc định: `~/.appstoreconnect/private_keys/AuthKey_883W4Z3Z99.p8`; dùng key khác
   thì thêm `ASC_KEY_ID=... ASC_KEY_PATH=...`.
4. Trên ASC UI còn 4 thứ deliver không đụng được: **Pricing** (Free), bảng câu hỏi
   **App Privacy** (chọn Data Not Collected), **Age rating** (None hết → 4+), và
   **số điện thoại** trong App Review Information.
5. Làm 4 việc code blocker (#3–#6 mục 8) + demo mode (#10) → build MAS → upload .pkg.
6. Chụp screenshots từ bản chạy thật (mục 6), upload.
7. Điền demo access vào Review Notes + video (mục 7) → **Submit for Review**.
