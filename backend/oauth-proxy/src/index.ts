/**
 * Widget for Jira — Atlassian OAuth 2.0 (3LO) token-exchange proxy.
 *
 * Stateless Cloudflare Worker. Atlassian's 3LO flow requires a
 * client_secret and does not support PKCE, so this worker holds the
 * secret server-side and the desktop (Tauri) app never sees it.
 * Nothing is persisted; tokens, authorization codes, and secrets are
 * never stored and never logged.
 */

export interface Env {
  CLIENT_ID: string;
  CLIENT_SECRET: string;
  REDIRECT_URI?: string;
}

const ATLASSIAN_TOKEN_URL = "https://auth.atlassian.com/oauth/token";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const { pathname } = url;

    if (request.method === "POST" && pathname === "/oauth/token") {
      return handleToken(request, env);
    }
    if (request.method === "GET" && pathname === "/oauth/callback") {
      return handleCallback(url);
    }
    if (request.method === "GET" && pathname === "/") {
      return html(infoPage(), 200);
    }
    return json({ error: "not_found" }, 404);
  },
};

/**
 * POST /oauth/token — exchange an authorization code or refresh token.
 *
 * The client body is never forwarded verbatim: a fresh, whitelisted
 * payload is assembled server-side and the client_id / client_secret
 * are injected here so the desktop app never handles the secret.
 */
async function handleToken(request: Request, env: Env): Promise<Response> {
  let raw: unknown;
  try {
    raw = await request.json();
  } catch {
    return json(
      { error: "invalid_request", error_description: "malformed JSON body" },
      400,
    );
  }
  if (typeof raw !== "object" || raw === null) {
    return json(
      { error: "invalid_request", error_description: "expected a JSON object" },
      400,
    );
  }

  const body = raw as Record<string, unknown>;
  const grantType = body.grant_type;

  // Whitelist the grant type to exactly the two flows we support.
  if (grantType !== "authorization_code" && grantType !== "refresh_token") {
    return json({ error: "unsupported_grant_type" }, 400);
  }

  // Only ever forward these fields — never spread the raw client body.
  const payload: Record<string, string> = {
    grant_type: grantType,
    client_id: env.CLIENT_ID,
    client_secret: env.CLIENT_SECRET,
  };

  if (grantType === "authorization_code") {
    const code = body.code;
    if (typeof code !== "string" || code.length === 0) {
      return json(
        { error: "invalid_request", error_description: "missing code" },
        400,
      );
    }
    payload.code = code;

    const provided = body.redirect_uri;
    const pinned = env.REDIRECT_URI;
    if (typeof pinned === "string" && pinned.length > 0) {
      // A pinned redirect URI is configured: any client-supplied value
      // must match it exactly, otherwise reject.
      if (typeof provided === "string" && provided !== pinned) {
        return json(
          {
            error: "invalid_request",
            error_description: "redirect_uri mismatch",
          },
          400,
        );
      }
      payload.redirect_uri = pinned;
    } else if (typeof provided === "string" && provided.length > 0) {
      // No pin configured: pass the client value through unchanged.
      payload.redirect_uri = provided;
    }
  } else {
    const refreshToken = body.refresh_token;
    if (typeof refreshToken !== "string" || refreshToken.length === 0) {
      return json(
        { error: "invalid_request", error_description: "missing refresh_token" },
        400,
      );
    }
    payload.refresh_token = refreshToken;
  }

  let upstream: Response;
  try {
    upstream = await fetch(ATLASSIAN_TOKEN_URL, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        accept: "application/json",
      },
      body: JSON.stringify(payload),
    });
  } catch {
    return json({ error: "upstream_unreachable" }, 502);
  }

  // Pass Atlassian's status and body through verbatim. The body is not
  // parsed or logged so no token material is ever inspected here.
  const passthrough = await upstream.text();
  return new Response(passthrough, {
    status: upstream.status,
    headers: { "content-type": "application/json" },
  });
}

/**
 * GET /oauth/callback — Atlassian redirects the browser here after the
 * user consents. We hand the code back to the desktop app's loopback
 * listener, whose port is carried in the state as `<nonce>.<port>`.
 */
function handleCallback(url: URL): Response {
  const params = url.searchParams;
  const error = params.get("error");
  const code = params.get("code");
  const state = params.get("state");

  // Never blind-redirect on an error or on anything malformed.
  if (error) {
    return html(errorPage(error), 200);
  }
  if (!code || !state) {
    return html(errorPage(null), 200);
  }

  const port = parseStatePort(state);
  if (port === null) {
    return html(errorPage(null), 200);
  }

  // Host is hardcoded to loopback; only the validated numeric port is
  // interpolated, and code/state are URL-encoded — no open redirect.
  const location =
    `http://127.0.0.1:${port}/cb` +
    `?code=${encodeURIComponent(code)}` +
    `&state=${encodeURIComponent(state)}`;
  return new Response(null, { status: 302, headers: { location } });
}

/**
 * Parse the loopback port out of the OAuth state (`<nonce>.<port>`).
 * Returns the port only when it is a decimal integer in 1024–65535.
 */
function parseStatePort(state: string): number | null {
  const lastDot = state.lastIndexOf(".");
  if (lastDot <= 0) return null; // require a non-empty nonce and a dot
  const portPart = state.slice(lastDot + 1);
  if (!/^\d+$/.test(portPart)) return null;
  const port = Number(portPart);
  if (port < 1024 || port > 65535) return null;
  return port;
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function json(body: unknown, status: number): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function html(markup: string, status: number): Response {
  return new Response(markup, {
    status,
    headers: { "content-type": "text/html; charset=utf-8" },
  });
}

function page(title: string, inner: string): string {
  return `<!doctype html>
<html lang="vi">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${title}</title>
<style>
  :root { color-scheme: light dark; }
  body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
    margin: 0; min-height: 100vh; display: grid; place-items: center;
    background: #f5f6f8; color: #1a1a1a; }
  main { max-width: 26rem; margin: 1.5rem; padding: 2rem;
    background: #fff; border-radius: 12px;
    box-shadow: 0 6px 24px rgba(0,0,0,.08); text-align: center; }
  h1 { font-size: 1.25rem; margin: 0 0 .5rem; }
  p { margin: .35rem 0; line-height: 1.5; }
  .muted { opacity: .7; font-size: .9rem; }
  code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
  @media (prefers-color-scheme: dark) {
    body { background: #16181d; color: #e6e6e6; }
    main { background: #21242b; box-shadow: 0 6px 24px rgba(0,0,0,.4); }
  }
</style>
</head>
<body>
<main>
${inner}
</main>
</body>
</html>`;
}

function infoPage(): string {
  return page(
    "Widget for Jira OAuth proxy",
    `<h1>Widget for Jira OAuth proxy</h1>
<p>Dịch vụ trung gian đổi mã OAuth cho app Widget for Jira.</p>
<p class="muted">No data stored — không lưu token, không lưu mã, không CORS.</p>`,
  );
}

function errorPage(errorCode: string | null): string {
  const detail = errorCode
    ? `<p class="muted">Mã lỗi: <code>${escapeHtml(errorCode)}</code></p>`
    : "";
  return page(
    "Đăng nhập không thành công",
    `<h1>Đăng nhập không thành công</h1>
<p>Quay lại app Widget for Jira và thử lại.</p>
${detail}`,
  );
}
