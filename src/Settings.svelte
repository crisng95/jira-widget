<script lang="ts">
  // Cua so cai dat. Moi thao tac ghi (config, Keychain) deu nam o Rust;
  // o day chi la form.
  //
  // Token: `type=password`, XOA khoi state ngay sau khi gui sang Rust, va
  // khong bao gio doc nguoc ra — UI chi biet "da co token" hay "chua".
  //
  // Chia 4 tab thay vi mot trang cuon dai: ban truoc dai ~15 truong lien tuc,
  // moi truong keo theo mot doan giai thich, nhin vao khong biet bat dau tu dau.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { t, setLang } from "./i18n.svelte";

  type Notify = {
    statusChanged: boolean;
    assigneeChanged: boolean;
    added: boolean;
    removed: boolean;
    groupThreshold: number;
  };
  type Dto = {
    jiraUrl: string;
    projectKey: string;
    boardId: number;
    me: string;
    pollIntervalSecs: number;
    staleDays: number;
    endingSoonHours: number;
    oldAgeDays: number;
    testStatuses: string[];
    showTestQueue: boolean;
    reviewStatuses: string[];
    showReviewQueue: boolean;
    pendingReleaseStatuses: string[];
    showReleaseQueue: boolean;
    windowLayer: string;
    /** "team" | "only_me" */
    displayMode: string;
    /** "vi" | "en" */
    language: string;
    /** "dc_pat" | "cloud_basic" | "cloud_oauth" */
    authMode: string;
    email: string;
    cloudId: string;
    notify: Notify;
    hasToken: boolean;
    hasOauth: boolean;
    oauthAvailable: boolean;
  };
  type Board = { id: number; name: string; boardType: string };
  type CloudSite = { id: string; url: string; name: string };
  type Msg = { kind: "ok" | "err" | "warn"; text: string } | null;

  const TAB_IDS = ["conn", "scope", "alert", "view"] as const;
  const TAB_LABELS: Record<(typeof TAB_IDS)[number], string> = {
    conn: "tabConn",
    scope: "tabScope",
    alert: "tabAlert",
    view: "tabView",
  };

  let tab = $state<(typeof TAB_IDS)[number]>("conn");
  let cfg = $state<Dto | null>(null);
  let boards = $state<Board[]>([]);
  let token = $state("");
  let msg = $state<Msg>(null);
  let busy = $state("");
  /** Site chua chon sau khi dang nhap OAuth co nhieu site */
  let sites = $state<CloudSite[]>([]);
  let testText = $state("");
  let reviewText = $state("");
  let releaseText = $state("");
  // Danh sach status THAT cua project. Rong => chua tai duoc, rot ve go tay.
  let allStatuses = $state<string[]>([]);

  // onMount PHAI dong bo: callback async tra ve Promise, Svelte khong coi do
  // la ham don dep nen listener se khong bao gio duoc go.
  onMount(() => {
    // Mode đổi được từ menu bar hoặc chip trên panel trong lúc cửa sổ này đang
    // mở. Không nghe thì seg hiện một đằng, panel chạy một nẻo — rồi bấm Lưu
    // là ghi đè ngược lại cái vừa chọn.
    const un = listen<string>("panel://display-mode", (ev) => {
      if (cfg) cfg.displayMode = ev.payload;
      modeDaAp = ev.payload;
    });
    // Cung ly do voi ngon ngu (wizard hay cua so khac co the doi).
    const unLang = listen<string>("panel://language", (ev) => {
      if (cfg) cfg.language = ev.payload;
      setLang(ev.payload);
    });

    void (async () => {
      try {
        const d = await invoke<Dto>("settings_get");
        cfg = d;
        setLang(d.language);
        modeDaAp = d.displayMode;
        testText = d.testStatuses.join("\n");
        reviewText = d.reviewStatuses.join("\n");
        releaseText = d.pendingReleaseStatuses.join("\n");
        // O mode OAuth thi PAT/API token khong lien quan — dung doa nguoi dung.
        if (!d.hasToken && d.authMode !== "cloud_oauth") {
          msg = { kind: "warn", text: t("noTokenYet") };
        }
        void loadBoards(true);
        void loadStatuses(true);
      } catch (e) {
        msg = { kind: "err", text: t("cantReadSettings", { e: String(e) }) };
      }
    })();

    return () => {
      un.then((f) => f());
      unLang.then((f) => f());
    };
  });

  // Mode KHONG di qua nut Luu. No la thu duy nhat (cung ngon ngu) doi nong
  // duoc, nen bam la ap dung luon. Giu rieng mode DA duoc backend chap nhan
  // de roll back seg khi lenh that bai.
  let modeDaAp = $state("team");

  async function doiMode(m: string) {
    if (!cfg) return;
    try {
      // Rust tra ve mode THUC SU duoc ap dung — no co the tu ha ve "team".
      const thuc = await invoke<string>("set_display_mode", { mode: m });
      cfg.displayMode = thuc;
      modeDaAp = thuc;
    } catch (e) {
      cfg.displayMode = modeDaAp; // seg phai quay ve dung cai panel dang chay
      msg = { kind: "err", text: t("modeChangeFail", { e: String(e) }) };
    }
  }

  // Ngon ngu cung doi nong nhu mode: bam la ap dung cho panel + cua so nay.
  async function doiNgonNgu(l: string) {
    if (!cfg) return;
    try {
      const eff = await invoke<string>("set_language", { lang: l });
      cfg.language = eff;
      setLang(eff);
    } catch (e) {
      msg = { kind: "err", text: `${e}` };
    }
  }

  // OAuth trong Cai dat: dang nhap lai / doi tai khoan. Site chon xong ghi
  // thang vao form (jiraUrl + cloudId) — van phai bam Luu de chot nhu moi field.
  async function oauthLoginS() {
    if (!cfg) return;
    busy = "oauth";
    sites = [];
    try {
      const r = await invoke<{ sites: CloudSite[] }>("oauth_begin");
      cfg.authMode = "cloud_oauth";
      cfg.hasOauth = true;
      if (r.sites.length === 1) {
        await chonSiteS(r.sites[0]);
      } else {
        sites = r.sites;
      }
    } catch (e) {
      msg = { kind: "err", text: t("loginFail", { e: String(e) }) };
    } finally {
      busy = "";
    }
  }

  async function chonSiteS(s: CloudSite) {
    if (!cfg) return;
    cfg.cloudId = s.id;
    cfg.jiraUrl = s.url;
    sites = [];
    busy = "whoami";
    try {
      const w = await invoke<{ name: string; displayName: string }>("oauth_whoami", {
        cloudId: s.id,
      });
      if (!cfg.me.trim() && w.name) cfg.me = w.name;
      msg = { kind: "ok", text: t("connOk", { d: w.displayName, n: w.name }) };
    } catch (e) {
      msg = { kind: "err", text: t("loginFail", { e: String(e) }) };
    } finally {
      busy = "";
    }
  }

  async function oauthLogoutS() {
    if (!cfg) return;
    busy = "oauth";
    try {
      await invoke("oauth_logout");
      cfg.hasOauth = false;
      msg = { kind: "warn", text: t("loggedOut") };
    } catch (e) {
      msg = { kind: "err", text: `${e}` };
    } finally {
      busy = "";
    }
  }

  async function loadBoards(quiet = false) {
    if (!cfg) return;
    busy = "boards";
    try {
      boards = await invoke<Board[]>("settings_list_boards", {
        jiraUrl: cfg.jiraUrl,
        projectKey: cfg.projectKey,
        authMode: cfg.authMode,
        email: cfg.email.trim() || null,
        tokenOverride: token.trim() || null,
        cloudId: cfg.cloudId || null,
      });
      if (!quiet) {
        msg = { kind: "ok", text: t("foundBoards", { n: boards.length, p: cfg.projectKey }) };
      }
    } catch (e) {
      boards = [];
      if (!quiet) msg = { kind: "err", text: t("boardsFail", { e: String(e) }) };
    } finally {
      busy = "";
    }
  }

  async function loadStatuses(quiet = false) {
    if (!cfg) return;
    busy = quiet ? busy : "statuses";
    try {
      allStatuses = await invoke<string[]>("settings_project_statuses", {
        jiraUrl: cfg.jiraUrl,
        projectKey: cfg.projectKey,
        authMode: cfg.authMode,
        email: cfg.email.trim() || null,
        tokenOverride: token.trim() || null,
        cloudId: cfg.cloudId || null,
      });
      if (!quiet) {
        msg = { kind: "ok", text: t("foundStatuses", { n: allStatuses.length, p: cfg.projectKey }) };
      }
    } catch (e) {
      allStatuses = [];
      if (!quiet) msg = { kind: "err", text: t("statusesFail", { e: String(e) }) };
    } finally {
      if (!quiet) busy = "";
    }
  }

  function textOf(list: string): string {
    return list === "test" ? testText : list === "review" ? reviewText : releaseText;
  }

  function toggleStatus(list: string, name: string) {
    const cur = textOf(list).split("\n").map((x) => x.trim()).filter(Boolean);
    const next = cur.includes(name) ? cur.filter((x) => x !== name) : [...cur, name];
    const joined = next.join("\n");
    if (list === "test") testText = joined;
    else if (list === "review") reviewText = joined;
    else releaseText = joined;
  }

  function isPicked(text: string, name: string): boolean {
    return text.split("\n").map((x) => x.trim()).includes(name);
  }

  async function testConnection() {
    if (!cfg) return;
    busy = "test";
    try {
      const who = await invoke<{ name: string; displayName: string }>(
        "settings_test_connection",
        {
          jiraUrl: cfg.jiraUrl,
          authMode: cfg.authMode,
          email: cfg.email.trim() || null,
          tokenOverride: token.trim() || null,
          cloudId: cfg.cloudId || null,
        },
      );
      // Tien the dien luon username vao o `me` — khoi phai di tra cuu
      const filled = !cfg.me.trim() && !!who.name;
      if (filled) cfg.me = who.name;
      msg = {
        kind: "ok",
        text:
          t("connOk", { d: who.displayName, n: who.name }) +
          (filled ? t("connOkFilled") : ""),
      };
    } catch (e) {
      msg = { kind: "err", text: t("connFail", { e: String(e) }) };
    } finally {
      busy = "";
    }
  }

  async function saveToken() {
    if (!token.trim()) return;
    busy = "token";
    try {
      await invoke("settings_save_token", { token });
      token = ""; // xoa khoi bo nho JS ngay
      if (cfg) cfg.hasToken = true;
      msg = { kind: "ok", text: t("tokenSaved") };
    } catch (e) {
      msg = { kind: "err", text: t("tokenSaveFail", { e: String(e) }) };
    } finally {
      busy = "";
    }
  }

  async function clearToken() {
    busy = "token";
    try {
      await invoke("settings_clear_token");
      if (cfg) cfg.hasToken = false;
      msg = { kind: "warn", text: t("tokenCleared") };
    } catch (e) {
      msg = { kind: "err", text: `${e}` };
    } finally {
      busy = "";
    }
  }

  async function save(thenRestart: boolean) {
    if (!cfg) return;
    busy = "save";
    try {
      const dto: Dto = {
        ...cfg,
        testStatuses: testText.split("\n"),
        reviewStatuses: reviewText.split("\n"),
        pendingReleaseStatuses: releaseText.split("\n"),
      };
      await invoke("settings_save", { dto });
      if (thenRestart) {
        await invoke("settings_apply_restart");
      } else {
        msg = { kind: "ok", text: t("savedHint") };
      }
    } catch (e) {
      msg = { kind: "err", text: `${e}` };
    } finally {
      busy = "";
    }
  }
</script>

<div class="sw">
  <div class="sw-head">
    <h1>{t("settings")}</h1>
    <div class="tabs" role="tablist">
      {#each TAB_IDS as id (id)}
        <button role="tab" aria-selected={tab === id} onclick={() => (tab = id)}>
          {t(TAB_LABELS[id])}
        </button>
      {/each}
    </div>
  </div>

  <div class="sw-body">
    {#if msg}
      <div class="banner {msg.kind}">
        <span class="ic">{msg.kind === "ok" ? "✓" : "!"}</span>
        <span>{msg.text}</span>
      </div>
    {/if}

    {#if !cfg}
      <p class="muted">{t("loading")}</p>
    {:else if tab === "conn"}
      <div class="field">
        <span class="lbl">{t("authLabel")}</span>
        <div class="seg">
          <button
            aria-pressed={cfg.authMode === "dc_pat"}
            onclick={() => cfg && (cfg.authMode = "dc_pat")}>{t("authDc")}</button
          >
          <button
            aria-pressed={cfg.authMode === "cloud_basic"}
            onclick={() => cfg && (cfg.authMode = "cloud_basic")}>{t("authCloudBasic")}</button
          >
          {#if cfg.oauthAvailable || cfg.hasOauth}
            <button
              aria-pressed={cfg.authMode === "cloud_oauth"}
              onclick={() => cfg && (cfg.authMode = "cloud_oauth")}>{t("authCloudOauth")}</button
            >
          {/if}
        </div>
      </div>
      {#if cfg.authMode === "cloud_basic"}
        <p class="hint">{t("authHintCloudBasic")}</p>
      {/if}

      <div class="field">
        <label for="url">{t("jiraUrl")}</label>
        <input
          id="url"
          type="text"
          bind:value={cfg.jiraUrl}
          placeholder={cfg.authMode === "dc_pat"
            ? "https://jira.cong-ty.vn"
            : "https://cong-ty.atlassian.net"}
        />
      </div>

      {#if cfg.authMode === "cloud_oauth"}
        <div class="field top">
          <span class="lbl"></span>
          <div class="stack">
            {#if cfg.hasOauth}
              <div class="banner ok" style="margin-bottom:0">
                <span class="ic">✓</span>
                <span>{t("loggedInAtlassian")}{#if cfg.me} · <b>{cfg.me}</b>{/if}</span>
              </div>
            {:else}
              <div class="banner warn" style="margin-bottom:0">
                <span class="ic">!</span>
                <span>{t("notLoggedIn")}</span>
              </div>
            {/if}
            <div class="row">
              <button onclick={oauthLoginS} disabled={busy !== ""}>
                {busy === "oauth" ? t("loginWaiting") : t("loginAtlassian")}
              </button>
              {#if cfg.hasOauth}
                <button class="ghost" onclick={oauthLogoutS} disabled={busy !== ""}>
                  {t("logout")}
                </button>
              {/if}
            </div>
            {#if sites.length > 1}
              <p class="hint" style="grid-column:auto;margin:0">{t("pickSite")}</p>
              {#each sites as s (s.id)}
                <button class="siterow" onclick={() => chonSiteS(s)}>
                  <b>{s.name}</b>
                  <span>{s.url.replace(/^https?:\/\//, "")}</span>
                </button>
              {/each}
            {/if}
          </div>
        </div>
      {:else}
        {#if cfg.authMode === "cloud_basic"}
          <div class="field">
            <label for="em">{t("emailLabel")}</label>
            <input id="em" type="text" bind:value={cfg.email} placeholder={t("emailPlaceholder")} />
          </div>
        {/if}

        <div class="field top">
          <label for="tok">
            {cfg.authMode === "cloud_basic" ? t("apiTokenLabel") : t("accessToken")}
          </label>
          <div class="stack">
            <div class="row">
              <input
                id="tok"
                type="password"
                bind:value={token}
                placeholder={cfg.hasToken ? t("tokenPlaceholderHas") : t("tokenPlaceholderNew")}
                autocomplete="off"
              />
              <button onclick={saveToken} disabled={busy !== "" || !token.trim()}>{t("save")}</button>
            </div>
            <!-- Trang thai token va nut xoa nam o DONG RIENG, khong nhet giua doan van -->
            <div class="tokline">
              {#if cfg.hasToken}
                <span class="ok">{t("tokenSavedKeychain")}</span>
                <button class="link" onclick={clearToken} disabled={busy !== ""}>{t("delete")}</button>
              {:else}
                <span>{t("tokenNone")}</span>
              {/if}
            </div>
          </div>
        </div>
        <p class="hint">
          {cfg.authMode === "cloud_basic" ? t("cloudTokenHowTo") : t("tokenHowTo")}
        </p>
      {/if}

      <div class="field">
        <span class="lbl"></span>
        <button onclick={testConnection} disabled={busy !== ""} style="justify-self:start">
          {busy === "test" ? t("testing") : t("testConn")}
        </button>
      </div>
      <p class="hint">{t("testConnHint")}</p>
    {:else if tab === "scope"}
      <div class="field">
        <label for="pk">{t("projectKey")}</label>
        <div class="row">
          <input id="pk" type="text" bind:value={cfg.projectKey} placeholder="PROJ" />
          <button
            onclick={() => {
              void loadBoards();
              void loadStatuses(true);
            }}
            disabled={busy !== ""}
          >
            {busy === "boards" ? t("finding") : t("findBoards")}
          </button>
        </div>
      </div>

      <div class="field">
        <label for="bd">{t("board")}</label>
        {#if boards.length > 0}
          <select id="bd" bind:value={cfg.boardId}>
            {#each boards as b (b.id)}
              <option value={b.id}>{b.name} · {b.boardType} · id {b.id}</option>
            {/each}
          </select>
        {:else}
          <div class="numfield">
            <input id="bd" type="number" bind:value={cfg.boardId} min="1" />
            <span class="unit">{t("boardManualHint")}</span>
          </div>
        {/if}
      </div>
      <p class="hint">{t("boardHint")}</p>

      <div class="field">
        <label for="me">{t("username")}</label>
        <input id="me" type="text" bind:value={cfg.me} placeholder="vd: sam.hale" />
      </div>
      <p class="hint">{t("usernameHint")}</p>

      <div class="sep"></div>
      <p class="group-title">{t("queues")}</p>
      <p class="hint qhint">{t("queuesHint")}</p>

      {#each [{ id: "test", label: t("qTest"), show: "showTestQueue" }, { id: "review", label: t("qReview"), show: "showReviewQueue" }, { id: "release", label: t("qRelease"), show: "showReleaseQueue" }] as q (q.id)}
        <div class="qbox">
          <label class="qhead">
            <input
              type="checkbox"
              class="sw-toggle"
              checked={cfg[q.show as "showTestQueue"]}
              onchange={(e) =>
                cfg && ((cfg as any)[q.show] = (e.currentTarget as HTMLInputElement).checked)}
            />
            <span class="qname">{q.label}</span>
            <span class="qcount">{t("nStatus", { n: textOf(q.id).split("\n").filter((x) => x.trim()).length })}</span>
          </label>

          {#if cfg[q.show as "showTestQueue"]}
            {#if allStatuses.length > 0}
              <div class="checks stat">
                {#each allStatuses as st (st)}
                  <label>
                    <input
                      type="checkbox"
                      checked={isPicked(textOf(q.id), st)}
                      onchange={() => toggleStatus(q.id, st)}
                    />
                    {st}
                  </label>
                {/each}
              </div>
            {:else}
              <p class="hint qhint">
                {t("statusLoadFail")}
                <button class="link" onclick={() => loadStatuses()} disabled={busy !== ""}>
                  {t("retry")}</button
                >. {t("statusTypoWarn")}
              </p>
            {/if}
          {/if}
        </div>
      {/each}

    {:else if tab === "alert"}
      <div class="field">
        <label for="poll">{t("pollCycle")}</label>
        <div class="numfield">
          <input id="poll" type="number" bind:value={cfg.pollIntervalSecs} min="10" />
          <span class="unit">{t("seconds")}</span>
        </div>
      </div>

      <div class="field">
        <label for="stale">{t("staleAfter")}</label>
        <div class="numfield">
          <input id="stale" type="number" bind:value={cfg.staleDays} min="0" />
          <span class="unit">{t("daysRedAlert")}</span>
        </div>
      </div>

      <div class="field">
        <label for="soon">{t("endingSoon")}</label>
        <div class="numfield">
          <input id="soon" type="number" bind:value={cfg.endingSoonHours} min="0" />
          <span class="unit">{t("lastHours")}</span>
        </div>
      </div>

      <div class="field">
        <label for="old">{t("oldTicket")}</label>
        <div class="numfield">
          <input id="old" type="number" bind:value={cfg.oldAgeDays} min="0" />
          <span class="unit">{t("daysOld")}</span>
        </div>
      </div>

      <div class="sep"></div>
      <p class="group-title">{t("notifications")}</p>

      <div class="field top">
        <span class="lbl">{t("notifyWhen")}</span>
        <div class="checks">
          <label><input type="checkbox" bind:checked={cfg.notify.statusChanged} /> {t("nStatusChanged")}</label>
          <label><input type="checkbox" bind:checked={cfg.notify.assigneeChanged} /> {t("nAssigneeChanged")}</label>
          <label><input type="checkbox" bind:checked={cfg.notify.added} /> {t("nAdded")}</label>
          <label><input type="checkbox" bind:checked={cfg.notify.removed} /> {t("nRemoved")}</label>
        </div>
      </div>

      <div class="field">
        <label for="gt">{t("groupOver")}</label>
        <div class="numfield">
          <input id="gt" type="number" bind:value={cfg.notify.groupThreshold} min="1" />
          <span class="unit">{t("changesAtOnce")}</span>
        </div>
      </div>
    {:else}
      <div class="field">
        <span class="lbl">{t("language")}</span>
        <div class="seg">
          <button aria-pressed={cfg.language === "vi"} onclick={() => doiNgonNgu("vi")}>
            Tiếng Việt
          </button>
          <button aria-pressed={cfg.language === "en"} onclick={() => doiNgonNgu("en")}>
            English
          </button>
        </div>
      </div>
      <p class="hint">{t("langHint")}</p>

      <div class="field">
        <span class="lbl">{t("panelShows")}</span>
        <div class="seg">
          <button aria-pressed={cfg.displayMode === "team"} onclick={() => doiMode("team")}>
            {t("modeTeam")}
          </button>
          <button
            aria-pressed={cfg.displayMode === "only_me"}
            disabled={!cfg.me.trim()}
            onclick={() => doiMode("only_me")}
          >
            {t("modeMine")}
          </button>
        </div>
      </div>
      <p class="hint">
        {#if !cfg.me.trim()}{t("needUsername")}{:else}{t("modeHint")}{/if}
      </p>

      <div class="field">
        <span class="lbl">{t("panelLayer")}</span>
        <div class="seg">
          <button
            aria-pressed={cfg.windowLayer === "desktop"}
            onclick={() => cfg && (cfg.windowLayer = "desktop")}
          >
            {t("layerDesktop")}
          </button>
          <button
            aria-pressed={cfg.windowLayer === "floating"}
            onclick={() => cfg && (cfg.windowLayer = "floating")}
          >
            {t("layerFloating")}
          </button>
        </div>
      </div>
      <p class="hint">{t("layerHint")}</p>
    {/if}
  </div>

  <div class="sw-foot">
    <button class="primary" onclick={() => save(true)} disabled={busy !== ""}>
      {busy === "save" ? t("saving") : t("saveRestart")}
    </button>
    <button onclick={() => save(false)} disabled={busy !== ""}>{t("saveOnly")}</button>
    <span class="spacer"></span>
    <button class="ghost" onclick={() => invoke("settings_close")}>{t("close")}</button>
  </div>
</div>
