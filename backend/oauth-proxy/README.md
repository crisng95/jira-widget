# Master Jira — OAuth Proxy (Cloudflare Worker)

Proxy đổi token Atlassian OAuth 2.0 (3LO) cho app desktop **Master Jira** (Tauri).
Worker **stateless**, viết bằng TypeScript, **không có runtime dependency**.

---

## (a) App này làm gì & vì sao cần

Atlassian OAuth 2.0 (3LO) **bắt buộc dùng `client_secret`** và **không hỗ trợ PKCE**.
Vì vậy secret **không được** nằm trong app desktop (ai cũng có thể mở file/binary ra đọc).

Worker này đứng ở giữa:

- Giữ `CLIENT_SECRET` **phía server** (Cloudflare Secret). App desktop không bao giờ thấy secret.
- Nhận `code` / `refresh_token` từ app → gắn thêm `client_id` + `client_secret` → gọi
  `https://auth.atlassian.com/oauth/token` → **trả nguyên văn** kết quả về app.
- **Stateless**: không có database, không lưu token, không lưu `code`, **không log** dữ liệu nhạy cảm.
- Xử lý luôn `redirect_uri` callback: Atlassian gọi về `/oauth/callback`, worker chuyển tiếp
  `code` về listener loopback của app (`http://127.0.0.1:<port>/cb`).

### Các endpoint

| Method | Path              | Mô tả                                                                 |
| ------ | ----------------- | --------------------------------------------------------------------- |
| `POST` | `/oauth/token`    | Đổi `authorization_code` hoặc `refresh_token` lấy token (JSON in/out) |
| `GET`  | `/oauth/callback` | Nhận callback từ Atlassian, redirect `code` về app qua loopback       |
| `GET`  | `/`               | Trang thông tin nhỏ ("no data stored")                                |
| khác   | —                 | `404 JSON`                                                             |

`state` mang theo port loopback theo định dạng `<nonce>.<port>` (port thập phân 1024–65535).

---

## (b) Đăng ký OAuth 2.0 (3LO) app

Vào **Atlassian Developer Console**: <https://developer.atlassian.com/console/myapps/>

1. **Create** → chọn **OAuth 2.0 integration**.
2. **Authorization** → thêm callback URL (đúng bằng domain của worker):

   ```
   https://<worker-domain>/oauth/callback
   ```

3. **Permissions** → thêm **Jira API** và bật các scope classic:

   - `read:jira-work`
   - `read:jira-user`

4. `offline_access` **không** khai ở màn Permissions — nó được **bật qua `scope` lúc authorize**
   để lấy `refresh_token` (xem URL authorize bên dưới).
5. Lấy **Client ID** và **Secret** ở tab **Settings**.

### URL authorize (app desktop tự dựng)

```
https://auth.atlassian.com/authorize?audience=api.atlassian.com
  &client_id=<CLIENT_ID>
  &scope=read:jira-work%20read:jira-user%20offline_access
  &redirect_uri=https://<worker-domain>/oauth/callback
  &state=<nonce>.<port>
  &response_type=code
  &prompt=consent
```

- `offline_access` trong `scope` → Atlassian trả về `refresh_token`.
- `state = <nonce>.<port>`: `nonce` để chống CSRF; `port` là cổng listener loopback của app.

---

## (c) Deploy

```bash
cd backend/oauth-proxy
npm i

# Nhập secret (không commit vào repo):
npx wrangler secret put CLIENT_ID
npx wrangler secret put CLIENT_SECRET

# (Tuỳ chọn) ghim redirect_uri — mở [vars] trong wrangler.toml và đặt:
#   REDIRECT_URI = "https://<worker-domain>/oauth/callback"

npm run deploy
```

Kiểm tra type trước khi deploy: `npm run check` (chạy `tsc --noEmit`).

> **REDIRECT_URI** là biến tuỳ chọn. Khi được đặt, `/oauth/token` sẽ **bắt buộc** mọi
> `redirect_uri` do client gửi lên phải **khớp tuyệt đối** với giá trị này (lệch → `400`).

---

## (d) Nối vào app Master Jira

Điền cấu hình vào file config của app:

`~/Library/Application Support/jira-widget/config.toml`

```toml
oauth_client_id  = "<CLIENT_ID>"
oauth_backend_url = "https://<worker-domain>"
```

Hoặc set biến môi trường **lúc build**:

```bash
export MASTERJIRA_OAUTH_CLIENT_ID="<CLIENT_ID>"
export MASTERJIRA_OAUTH_BACKEND_URL="https://<worker-domain>"
```

> `oauth_backend_url` là **origin** của worker (không kèm path). App tự ghép
> `/oauth/token` và dùng `/oauth/callback` làm `redirect_uri`.

---

## (e) Ghi chú bản Mac App Store (sandbox)

App mở một listener loopback (`127.0.0.1:<port>`) để nhận `code` từ callback.
Trong sandbox, việc lắng nghe socket cần entitlement:

```xml
<key>com.apple.security.network.server</key>
<true/>
```

(Ngoài ra `com.apple.security.network.client` cho các request ra ngoài.)
Thiếu `network.server`, listener loopback sẽ **không nhận được** callback.

---

## (f) Bảo mật

- **Whitelist `grant_type`**: chỉ chấp nhận đúng `authorization_code` hoặc `refresh_token`;
  loại khác → `400 {"error":"unsupported_grant_type"}`.
- **Không forward body thô**: worker tự dựng payload trắng danh sách (chỉ các field cần) rồi mới
  chèn `client_id` + `client_secret` phía server — client không thể nhét thêm tham số.
- **Không log token**: không `console.log` body, không đọc/parse nội dung token khi passthrough.
- **Không CORS**: client là app native (không phải XHR trình duyệt) nên worker không set header CORS.
- **Callback không blind-redirect**: nếu Atlassian trả `error`, hoặc `state`/port sai định dạng →
  hiện trang HTML lỗi, **không** redirect mù.
- **Port callback bị giới hạn 1024–65535**: host redirect hardcode `127.0.0.1`, chỉ chèn port đã
  validate là số nguyên trong dải; `code`/`state` được URL-encode → không có lỗ hổng open redirect.
- **Escape HTML**: mọi giá trị nội suy vào trang lỗi đều được HTML-escape.

### Khuyến nghị thêm cho production

- **Đặt `REDIRECT_URI`** (vars trong `wrangler.toml` hoặc dashboard) = chính xác
  `https://<worker-domain>/oauth/callback`. Khi đặt, mọi `redirect_uri` client gửi lên
  mà lệch sẽ bị `400` — khoá chặt thêm một nấc chống lạm dụng `client_id`.
- **Bật rate limiting** cho route `POST /oauth/token` (Cloudflare dashboard → Security →
  Rate limiting rules, ví dụ 10 req/phút/IP). Worker là relay không trạng thái và không
  có auth riêng — không giới hạn thì bên thứ ba có thể mượn `client_id` của app để dò mã
  hoặc đốt quota consent của Atlassian.
