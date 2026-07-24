# Privacy Policy — Master Jira

**Effective date:** 24 July 2026
**Developer:** ISEMI COMPANY LIMITED — contact: aabooksapp@gmail.com

## Summary

This app does **not** collect, store, transmit, or sell any personal data to the
developer or to any third party. There are no analytics, no advertising SDKs, no
crash reporting, and no tracking of any kind. All data stays on your Mac or
travels only between your Mac and the Jira server **you** configure.

## What the app does with data

- **Jira data.** The app connects directly from your Mac to the Jira
  Server/Data Center URL you enter in Settings, authenticated with your own
  Personal Access Token, and reads sprint, board, and issue data through the
  Jira REST API over HTTPS. That data is used solely to render the panel on
  your screen and to show local notifications. It is never sent anywhere other
  than back to your own Jira server.
- **Your Personal Access Token.** Stored in the macOS Keychain on your device.
  It is transmitted only to the Jira server you configured, as an
  `Authorization` header over HTTPS. The developer never sees or receives it.
- **Configuration.** Settings (Jira URL, board id, display thresholds, …) are
  stored locally in
  `~/Library/Application Support/jira-widget/config.toml`.
- **Logs.** Diagnostic logs are written locally to
  `~/Library/Logs/jira-widget.log` and are never uploaded.

## Network connections

The only network destination is the Jira server you configure. The app makes no
other connections — no update pings, no telemetry, no third-party services.

## Notifications

The app posts local macOS notifications about changes in your sprint (status
changes, reassignments, tickets added/removed). Notification content is
generated entirely on your device.

## Data deletion

- Remove the token: Settings → clear token, or run `jira-widget --clear-token`,
  or delete the `jira-widget` item in Keychain Access.
- Remove all local data: delete the app, the folder
  `~/Library/Application Support/jira-widget/`, and the log file above.

## Children's privacy

The app is a workplace productivity tool and does not knowingly collect any
data from anyone, including children.

## Changes to this policy

If the app ever changes what it does with data, this document will be updated
and the effective date revised before that version ships.

---

# Chính sách quyền riêng tư (tiếng Việt — bản tóm tắt)

Ứng dụng **không thu thập bất kỳ dữ liệu nào** về máy chủ của nhà phát triển
hay bên thứ ba: không analytics, không quảng cáo, không tracking. Dữ liệu Jira
chỉ đi thẳng giữa máy của bạn và máy chủ Jira do chính bạn cấu hình, qua HTTPS.
Personal Access Token nằm trong Keychain của macOS; nhà phát triển không bao
giờ nhìn thấy nó. Cấu hình và log đều lưu cục bộ trên máy. Xoá app + thư mục
`~/Library/Application Support/jira-widget/` + token trong Keychain là xoá
sạch mọi dữ liệu.

Liên hệ: aabooksapp@gmail.com
