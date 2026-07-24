<script lang="ts">
  // Wizard chao mung lan dau — nam buoc ep thu tu, nut "Tiep" khoa toi khi
  // buoc do hop le. Chi chay khi CHUA co token; nguoi dung cu khong thay no.
  //
  // Moi thao tac ghi (config, Keychain) van nam o Rust qua cac lenh settings_*
  // — wizard chi la mot lop dan duong khac tren cung bo lenh voi Cai dat.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { t, setLang, osLang, type Lang } from "./i18n.svelte";

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
    displayMode: string;
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

  const STEP_KEYS = ["wizStepLang", "wizStepConn", "wizStepBoard", "wizStepView", "wizStepDone"];

  // VI/EN dung day du; phan con lai la ban nhap can soat nen khoa lai.
  const LANGS = [
    { id: "vi", flag: "🇻🇳", name: "Tiếng Việt", ready: true },
    { id: "en", flag: "🇬🇧", name: "English", ready: true },
    { id: "zh", flag: "🇨🇳", name: "中文", sub: "简体", ready: false },
    { id: "ja", flag: "🇯🇵", name: "日本語", ready: false },
    { id: "ko", flag: "🇰🇷", name: "한국어", ready: false },
    { id: "fr", flag: "🇫🇷", name: "Français", ready: false },
  ];

  let step = $state(1);
  let cfg = $state<Dto | null>(null);
  let lang = $state<Lang>("vi");
  let jiraUrl = $state("");
  let token = $state("");
  let hasToken = $state(false);
  let who = $state<{ name: string; displayName: string } | null>(null);
  let connOk = $state(false);
  let projectKey = $state("");
  let boards = $state<Board[]>([]);
  let boardId = $state(0);
  // Danh sach project fetch bang credential vua nhap — chon thay vi go tay.
  let projects = $state<{ key: string; name: string }[]>([]);
  let projSearch = $state("");
  let projectsFail = $state(false);
  let me = $state("");
  let mode = $state("team");
  let busy = $state("");
  let err = $state("");

  // --- auth ---
  /** "dc_pat" | "cloud_basic" | "cloud_oauth" */
  let authMode = $state("dc_pat");
  let email = $state("");
  let oauthAvailable = $state(false);
  /** Tai khoan Atlassian co nhieu site thi phai chon mot */
  let sites = $state<CloudSite[]>([]);
  let cloudId = $state("");

  let canNext = $derived.by(() => {
    if (step === 2) return connOk;
    if (step === 3) return boardId > 0 && boards.some((b) => b.id === boardId);
    return true;
  });

  let boardName = $derived(boards.find((b) => b.id === boardId)?.name ?? "");
  let modeThucTe = $derived(mode === "only_me" && me.trim() ? "only_me" : "team");

  onMount(() => {
    // Ngon ngu mac dinh doan theo he dieu hanh — nguoi dung chot lai o buoc 1.
    lang = osLang();
    setLang(lang);
    void (async () => {
      try {
        const d = await invoke<Dto>("settings_get");
        cfg = d;
        // Config mac dinh mang gia tri vi du — khong prefill rac vao form.
        if (!d.jiraUrl.includes("example.com")) jiraUrl = d.jiraUrl;
        if (d.projectKey !== "PROJ") projectKey = d.projectKey;
        me = d.me;
        hasToken = d.hasToken;
        authMode = d.authMode === "cloud_oauth" && !d.hasOauth ? "dc_pat" : d.authMode;
        email = d.email;
        cloudId = d.cloudId;
        oauthAvailable = d.oauthAvailable;
      } catch (e) {
        err = String(e);
      }
    })();
  });

  // Go URL *.atlassian.net o che do DC la nham 100% — tu chuyen sang Cloud.
  function urlDoi() {
    connDirty();
    if (jiraUrl.includes(".atlassian.net") && authMode === "dc_pat") {
      authMode = "cloud_basic";
    }
  }

  function doiAuthMode(m: string) {
    if (authMode === m) return;
    authMode = m;
    connDirty();
  }

  // "Login with Atlassian": mo browser, cho callback. Xong thi refresh token
  // da nam trong Keychain; con lai la chon site va hoi "toi la ai".
  async function oauthLogin() {
    busy = "oauth";
    err = "";
    sites = [];
    try {
      const r = await invoke<{ sites: CloudSite[] }>("oauth_begin");
      authMode = "cloud_oauth";
      token = "";
      if (r.sites.length === 1) {
        await chonSite(r.sites[0]);
      } else {
        sites = r.sites; // nguoi dung chon, chonSite() lo phan con lai
      }
    } catch (e) {
      err = t("loginFail", { e: String(e) });
    } finally {
      busy = "";
    }
  }

  async function chonSite(s: CloudSite) {
    cloudId = s.id;
    jiraUrl = s.url;
    // site khac -> project/board khac
    projects = [];
    projectsFail = false;
    projSearch = "";
    boards = [];
    boardId = 0;
    busy = "whoami";
    err = "";
    try {
      const w = await invoke<{ name: string; displayName: string }>("oauth_whoami", {
        cloudId: s.id,
      });
      who = w;
      // `me` tren Cloud la accountId — dien tu dang nhap, khong bat go tay.
      if (w.name) me = w.name;
      connOk = true;
    } catch (e) {
      err = t("loginFail", { e: String(e) });
      connOk = false;
    } finally {
      busy = "";
    }
  }

  async function pickLang(l: string) {
    lang = l as Lang;
    setLang(lang);
    // Ghi ngay de panel/Cai dat cung doi; loi o day khong chan wizard.
    try {
      await invoke("set_language", { lang: l });
    } catch (e) {
      console.error("set_language loi", e);
    }
  }

  // URL hay token doi la ket qua kiem tra cu het gia tri — keo theo ca
  // danh sach project/board da fetch bang credential cu.
  function connDirty() {
    connOk = false;
    who = null;
    projects = [];
    projectsFail = false;
    projSearch = "";
    boards = [];
    boardId = 0;
  }

  async function testConn() {
    busy = "test";
    err = "";
    try {
      const w = await invoke<{ name: string; displayName: string }>(
        "settings_test_connection",
        {
          jiraUrl: jiraUrl.trim(),
          authMode,
          email: email.trim() || null,
          tokenOverride: token.trim() || null,
          cloudId: cloudId || null,
        },
      );
      who = w;
      if (!me.trim() && w.name) me = w.name;
      // Token vua go va da xac thuc -> cat vao Keychain ngay, xoa khoi JS.
      if (token.trim()) {
        await invoke("settings_save_token", { token });
        token = "";
        hasToken = true;
      }
      // Checkpoint URL + auth mode vao config NGAY khi da xac thuc: cac buoc
      // sau (tim board, liet ke status) dung token DA LUU, ma token da luu chi
      // duoc phep di toi host da luu — khong ghi lai thi buoc 3 bi chan oan.
      if (cfg) {
        const checkpoint: Dto = {
          ...cfg,
          jiraUrl: jiraUrl.trim(),
          authMode,
          email: email.trim(),
          hasToken,
        };
        await invoke("settings_save", { dto: checkpoint });
        cfg = checkpoint;
      }
      connOk = true;
    } catch (e) {
      err = t("connFail", { e: String(e) });
      connOk = false;
    } finally {
      busy = "";
    }
  }

  async function fetchProjects() {
    busy = "projects";
    err = "";
    projectsFail = false;
    try {
      projects = await invoke<{ key: string; name: string }[]>("settings_list_projects", {
        jiraUrl: jiraUrl.trim(),
        authMode,
        email: email.trim() || null,
        tokenOverride: null,
        cloudId: cloudId || null,
      });
      // Chi co dung mot project thi chon luon, khoi bat bam them mot cu.
      if (projects.length === 1) {
        projectKey = projects[0].key;
        void findBoards();
      }
    } catch (e) {
      projects = [];
      projectsFail = true;
      err = t("projectsFail", { e: String(e) });
    } finally {
      busy = "";
    }
  }

  function chonProject(p: { key: string; name: string }) {
    if (projectKey === p.key) return;
    projectKey = p.key;
    boards = [];
    boardId = 0;
    void findBoards();
  }

  // Loc client-side: danh sach da fetch het mot lan, go toi dau loc toi do.
  let projLoc = $derived.by(() => {
    const q = projSearch.trim().toLowerCase();
    const list = q
      ? projects.filter((p) => `${p.key} ${p.name}`.toLowerCase().includes(q))
      : projects;
    return list.slice(0, 8);
  });

  async function findBoards() {
    busy = "boards";
    err = "";
    try {
      boards = await invoke<Board[]>("settings_list_boards", {
        jiraUrl: jiraUrl.trim(),
        projectKey: projectKey.trim(),
        authMode,
        email: email.trim() || null,
        tokenOverride: null,
        cloudId: cloudId || null,
      });
      if (boards.length === 1) boardId = boards[0].id;
    } catch (e) {
      boards = [];
      err = t("boardsFail", { e: String(e) });
    } finally {
      busy = "";
    }
  }

  function goBack() {
    if (step > 1) {
      step -= 1;
      err = "";
    }
  }

  function goNext() {
    if (!canNext || step >= 5) return;
    step += 1;
    err = "";
    if (step === 3) {
      // Fetch danh sach project bang credential vua xac thuc — nguoi dung
      // chon tu danh sach, khong phai go key tay.
      if (projects.length === 0 && !projectsFail) {
        void fetchProjects();
      }
      // Da co key tu truoc (chay lai wizard) thi tim board luon.
      if (projectKey.trim() && boards.length === 0) {
        void findBoards();
      }
    }
  }

  async function start() {
    if (!cfg) return;
    busy = "start";
    err = "";
    try {
      const dto: Dto = {
        ...cfg,
        jiraUrl: jiraUrl.trim(),
        projectKey: projectKey.trim().toUpperCase(),
        boardId,
        me: me.trim(),
        authMode,
        email: email.trim(),
        cloudId,
        hasToken,
      };
      await invoke("settings_save", { dto });
      // display_mode khong di qua merge cua settings_save (no doi nong).
      // Cung khong dung duoc set_display_mode: no doi chieu voi cfg.me cua
      // phien dang chay — luc nay van rong. `onboarding_finish` ghi thang
      // xuong dia roi khoi dong lai.
      await invoke("onboarding_finish", { mode: modeThucTe });
    } catch (e) {
      err = t("wizSaveFail", { e: String(e) });
      busy = "";
    }
  }
</script>

<div class="wiz">
  <div class="wiz-body">
    <div class="steps">
      {#each STEP_KEYS as k, i (k)}
        {#if i > 0}<span class="rail" class:on={step > i}></span>{/if}
        <div class="st" class:active={step === i + 1} class:done={step > i + 1}>
          <span class="no num">{step > i + 1 ? "✓" : i + 1}</span>
          {#if step === i + 1}<span class="tx">{t(k)}</span>{/if}
        </div>
      {/each}
    </div>

    {#if err}
      <div class="banner err"><span class="ic">!</span><span>{err}</span></div>
    {/if}

    {#if step === 1}
      <h3 class="wiz-h">{t("wizLangH")}</h3>
      <p class="wiz-sub">{t("wizLangSub")}</p>
      <div class="lang-grid">
        {#each LANGS as l (l.id)}
          <button
            class="lang"
            class:sel={lang === l.id}
            disabled={!l.ready}
            onclick={() => pickLang(l.id)}
          >
            <span class="fl">{l.flag}</span>
            <span class="ln">
              {l.name}
              {#if l.id === osLang()}<small>{t("wizLangAuto")}</small>
              {:else if l.sub}<small>{l.sub}</small>{/if}
            </span>
            {#if !l.ready}
              <span class="draft">{t("wizDraft")}</span>
            {:else}
              <span class="mk" class:show={lang === l.id}>✓</span>
            {/if}
          </button>
        {/each}
      </div>
    {:else if step === 2}
      <h3 class="wiz-h">{t("wizConnH")}</h3>
      <p class="wiz-sub">{t("wizConnSub")}</p>

      {#if connOk && who}
        <div class="banner ok">
          <span class="ic">✓</span>
          <span>{t("connOk", { d: who.displayName, n: who.name })}</span>
        </div>
      {/if}

      {#if oauthAvailable}
        <!-- Duong "khong can tao token": mot nut, browser lo phan con lai -->
        <button class="primary oauthbtn" onclick={oauthLogin} disabled={busy !== ""}>
          {busy === "oauth" ? t("loginWaiting") : t("loginAtlassian")}
        </button>

        {#if sites.length > 1}
          <p class="fh" style="margin-bottom:6px">{t("pickSite")}</p>
          {#each sites as s (s.id)}
            <button class="board" class:sel={cloudId === s.id} onclick={() => chonSite(s)}>
              <span class="rd"></span>
              <span class="binfo">
                <span class="bn">{s.name}</span>
                <span class="bt">{s.url.replace(/^https?:\/\//, "")}</span>
              </span>
            </button>
          {/each}
        {/if}

        <div class="divider"><span>{t("orManualToken")}</span></div>
      {/if}

      {#if authMode !== "cloud_oauth" || !connOk}
        <div class="field col">
          <span class="lbl2">{t("authLabel")}</span>
          <div class="seg">
            <button
              aria-pressed={authMode === "dc_pat"}
              onclick={() => doiAuthMode("dc_pat")}>{t("authDc")}</button
            >
            <button
              aria-pressed={authMode === "cloud_basic"}
              onclick={() => doiAuthMode("cloud_basic")}>{t("authCloudBasic")}</button
            >
          </div>
        </div>

        <div class="field col">
          <label for="wurl">{t("jiraUrl")} <span class="req">*</span></label>
          <input
            id="wurl"
            type="text"
            bind:value={jiraUrl}
            oninput={urlDoi}
            placeholder={authMode === "cloud_basic"
              ? "https://cong-ty.atlassian.net"
              : "https://jira.cong-ty.vn"}
            autocomplete="off"
          />
        </div>
        {#if authMode === "cloud_basic"}
          <div class="field col">
            <label for="wem">{t("emailLabel")} <span class="req">*</span></label>
            <input
              id="wem"
              type="text"
              bind:value={email}
              oninput={connDirty}
              placeholder={t("emailPlaceholder")}
              autocomplete="off"
            />
          </div>
        {/if}
        <div class="field col">
          <label for="wtok">
            {authMode === "cloud_basic" ? t("apiTokenLabel") : t("patLabel")}
            {#if !hasToken}<span class="req">*</span>{/if}
          </label>
          <input
            id="wtok"
            type="password"
            bind:value={token}
            oninput={connDirty}
            placeholder={hasToken ? t("tokenPlaceholderHas") : t("tokenPlaceholderNew")}
            autocomplete="off"
          />
          <p class="fh">
            {authMode === "cloud_basic" ? t("cloudTokenHowTo") : t("tokenHowTo")}
            {t("wizTokenKeychain")}
          </p>
        </div>
        {#if !connOk}
          <p class="fh">{t("wizConnNeedTest")}</p>
        {/if}
      {/if}
    {:else if step === 3}
      <h3 class="wiz-h">{t("wizBoardH")}</h3>
      <p class="wiz-sub">{t("wizBoardSub")}</p>

      {#if projects.length > 0}
        <!-- Combobox: go de loc, bam de chon — key khong con phai nho -->
        <div class="field col">
          <label for="wps">{t("projectKey")} <span class="req">*</span></label>
          <input
            id="wps"
            type="text"
            bind:value={projSearch}
            placeholder={t("searchProject")}
            autocomplete="off"
          />
        </div>
        {#if projLoc.length === 0}
          <p class="fh">{t("noProjectMatch")}</p>
        {/if}
        {#each projLoc as p (p.key)}
          <button class="board" class:sel={projectKey === p.key} onclick={() => chonProject(p)}>
            <span class="rd"></span>
            <span class="binfo">
              <span class="bn">{p.key}</span>
              <span class="bt">{p.name}</span>
            </span>
          </button>
        {/each}
      {:else if busy === "projects"}
        <p class="fh">{t("loadingProjects")}</p>
      {:else}
        <!-- Fallback: khong tai duoc danh sach (mang/quyen) thi van go tay duoc -->
        <div class="field col">
          <label for="wpk">{t("projectKey")} <span class="req">*</span></label>
          <div class="row">
            <input id="wpk" type="text" bind:value={projectKey} placeholder="PROJ" />
            <button onclick={findBoards} disabled={busy !== "" || !projectKey.trim()}>
              {busy === "boards" ? t("finding") : t("findBoards")}
            </button>
          </div>
          <p class="fh">{t("projectManualHint")}</p>
        </div>
      {/if}

      {#if projectKey.trim() && (projects.length > 0 || boards.length > 0 || busy === "boards")}
        <div class="bsep"></div>
        {#if busy === "boards"}
          <p class="fh">{t("finding")}</p>
        {:else if boards.length === 0 && projects.length > 0}
          <p class="fh">{t("wizBoardEmpty")}</p>
        {:else}
          {#each boards as b (b.id)}
            <button class="board" class:sel={boardId === b.id} onclick={() => (boardId = b.id)}>
              <span class="rd"></span>
              <span class="binfo">
                <span class="bn">{b.name}</span>
                <span class="bt">{b.boardType}</span>
              </span>
              <span class="bid num">#{b.id}</span>
            </button>
          {/each}
        {/if}
      {:else if projects.length === 0 && busy !== "projects" && !projectsFail}
        <p class="fh">{t("wizBoardEmpty")}</p>
      {/if}
    {:else if step === 4}
      <h3 class="wiz-h">{t("wizViewH")}</h3>
      <p class="wiz-sub">{t("wizViewSub")}</p>

      <div class="field col">
        <label for="wme">{t("username")}</label>
        <input id="wme" type="text" bind:value={me} placeholder="vd: sam.hale" />
        <p class="fh">{t("usernameHint")}</p>
      </div>

      <div class="field col">
        <span class="lbl2">{t("wizViewMode")}</span>
        <div class="choice">
          <button class="opt" class:sel={mode === "team"} onclick={() => (mode = "team")}>
            <span class="ot">{t("wizModeTeamT")}</span>
            <span class="od">{t("wizModeTeamD")}</span>
          </button>
          <button
            class="opt"
            class:sel={mode === "only_me"}
            disabled={!me.trim()}
            onclick={() => (mode = "only_me")}
          >
            <span class="ot">{t("wizModeMineT")}</span>
            <span class="od">{t("wizModeMineD")}</span>
          </button>
        </div>
      </div>
    {:else}
      <h3 class="wiz-h">{t("wizDoneH")}</h3>
      <p class="wiz-sub">{t("wizDoneSub")}</p>

      <div class="summary">
        <div class="sr">
          <span class="sk">{t("wizSumLang")}</span>
          <span class="sv">{LANGS.find((l) => l.id === lang)?.flag} {LANGS.find((l) => l.id === lang)?.name}</span>
        </div>
        <div class="sr">
          <span class="sk">{t("wizSumJira")}</span>
          <span class="sv mono">{jiraUrl.replace(/^https?:\/\//, "") || t("wizNotSet")}</span>
        </div>
        <div class="sr">
          <span class="sk">{t("authLabel")}</span>
          <span class="sv">
            {authMode === "cloud_oauth"
              ? t("authCloudOauth")
              : authMode === "cloud_basic"
                ? t("authCloudBasic")
                : t("authDc")}
          </span>
        </div>
        <div class="sr">
          <span class="sk">{t("wizSumProject")}</span>
          <span class="sv">
            {projectKey.trim().toUpperCase()} · {boardName}
            <span class="mono num">#{boardId}</span>
          </span>
        </div>
        <div class="sr">
          <span class="sk">{t("wizSumMe")}</span>
          <span class="sv">{me.trim() || t("wizNotSet")}</span>
        </div>
        <div class="sr">
          <span class="sk">{t("wizSumView")}</span>
          <span class="sv">{modeThucTe === "only_me" ? t("wizSumViewMine") : t("wizSumViewTeam")}</span>
        </div>
      </div>
    {/if}
  </div>

  <div class="wiz-foot">
    {#if step > 1}
      <button class="ghost" onclick={goBack}>{t("wizBack")}</button>
    {/if}
    <span class="grow"></span>
    {#if step === 2 && authMode !== "cloud_oauth"}
      <button
        onclick={testConn}
        disabled={busy !== "" ||
          !jiraUrl.trim() ||
          (!hasToken && !token.trim()) ||
          (authMode === "cloud_basic" && !email.trim())}
      >
        {busy === "test" ? t("testing") : t("testConn")}
      </button>
    {/if}
    {#if step < 5}
      <button class="primary" onclick={goNext} disabled={!canNext}>{t("wizNext")}</button>
    {:else}
      <button class="primary wide" onclick={start} disabled={busy !== ""}>
        {busy === "start" ? t("wizStarting") : t("wizStart")}
      </button>
    {/if}
  </div>
</div>

<style>
  .wiz {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .wiz-body {
    flex: 1;
    overflow-y: auto;
    padding: 22px 26px;
  }
  .wiz-body::-webkit-scrollbar {
    width: 7px;
  }
  .wiz-body::-webkit-scrollbar-thumb {
    background: var(--baseline);
    border-radius: 4px;
  }

  /* ---- step indicator ---- */
  .steps {
    display: flex;
    align-items: center;
    margin-bottom: 22px;
  }
  .steps .st {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .steps .no {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    flex: none;
    display: grid;
    place-items: center;
    font-size: 11px;
    font-weight: 680;
    background: var(--raised);
    color: var(--text-muted);
  }
  .steps .tx {
    font-size: 11.5px;
    white-space: nowrap;
    color: var(--text-primary);
    font-weight: 620;
  }
  .steps .st.active .no {
    background: var(--series-1);
    color: #fff;
  }
  .steps .st.done .no {
    background: var(--you-bg);
    color: var(--you-fg);
  }
  .steps .rail {
    flex: 1;
    height: 2px;
    background: var(--raised);
    margin: 0 9px;
    border-radius: 2px;
    min-width: 12px;
  }
  .steps .rail.on {
    background: var(--series-1);
  }

  .wiz-h {
    font-size: 19px;
    font-weight: 680;
    letter-spacing: -0.015em;
    margin: 0 0 5px;
  }
  .wiz-sub {
    font-size: 13px;
    color: var(--text-muted);
    margin: 0 0 18px;
    max-width: 52ch;
  }

  .fh {
    font-size: 11.5px;
    color: var(--text-muted);
    margin: 6px 0 0;
  }

  /* ---- OAuth ---- */
  .oauthbtn {
    width: 100%;
    padding: 10px 16px;
    font-size: 13.5px;
    margin-bottom: 4px;
  }
  .divider {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 14px 0;
    font-size: 11px;
    color: var(--text-muted);
  }
  .divider::before,
  .divider::after {
    content: "";
    flex: 1;
    height: 1px;
    background: var(--gridline);
  }
  .lbl2 {
    display: block;
    font-size: 12.5px;
    font-weight: 560;
    color: var(--text-secondary);
    margin-bottom: var(--sp-md);
  }

  /* ---- buoc 1: ngon ngu ---- */
  .lang-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-lg);
  }
  .lang {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 10px;
    border: 0.5px solid var(--border);
    background: var(--raised);
    cursor: pointer;
    text-align: left;
    font-weight: 400;
  }
  .lang:disabled {
    opacity: 0.55;
  }
  .lang .fl {
    font-size: 18px;
  }
  .lang .ln {
    font-size: 12.5px;
    font-weight: 560;
    color: var(--text-primary);
  }
  .lang .ln small {
    display: block;
    font-weight: 400;
    color: var(--text-muted);
    font-size: 11px;
  }
  .lang.sel {
    border-color: var(--series-1);
    background: var(--you-bg);
    box-shadow: 0 0 0 1px var(--series-1) inset;
  }
  .lang .mk {
    margin-left: auto;
    color: var(--series-1);
    font-weight: 700;
    opacity: 0;
  }
  .lang .mk.show {
    opacity: 1;
  }
  .lang .draft {
    margin-left: auto;
    font-size: 9.5px;
    color: var(--text-muted);
    border: 0.5px solid var(--border);
    border-radius: 5px;
    padding: 1px 5px;
  }

  /* ---- buoc 3: board ---- */
  .bsep {
    height: 1px;
    background: var(--gridline);
    margin: 14px 0;
  }

  .board {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 11px;
    width: 100%;
    padding: 11px 13px;
    border-radius: 10px;
    border: 0.5px solid var(--border);
    background: var(--raised);
    margin-bottom: 7px;
    cursor: pointer;
    text-align: left;
    font-weight: 400;
  }
  .board .rd {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid var(--baseline);
    flex: none;
    background: transparent;
  }
  .board.sel {
    border-color: var(--series-1);
    background: var(--you-bg);
  }
  .board.sel .rd {
    border-color: var(--series-1);
    box-shadow: inset 0 0 0 3px var(--series-1);
  }
  .binfo {
    min-width: 0;
  }
  .board .bn {
    display: block;
    font-size: 13px;
    font-weight: 560;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .board .bt {
    display: block;
    font-size: 11px;
    color: var(--text-muted);
  }
  .board .bid {
    font-size: 11px;
    color: var(--text-muted);
  }

  /* ---- buoc 4: choice cards ---- */
  .choice {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 9px;
  }
  .choice .opt {
    padding: 13px;
    border-radius: 11px;
    border: 0.5px solid var(--border);
    background: var(--raised);
    cursor: pointer;
    text-align: left;
    font-weight: 400;
  }
  .choice .opt.sel {
    border-color: var(--series-1);
    background: var(--you-bg);
  }
  .choice .opt .ot {
    display: block;
    font-size: 12.5px;
    font-weight: 620;
    margin-bottom: 3px;
    color: var(--text-primary);
  }
  .choice .opt .od {
    display: block;
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  /* ---- buoc 5: summary ---- */
  .summary {
    display: flex;
    flex-direction: column;
    gap: 1px;
    border: 0.5px solid var(--border);
    border-radius: 11px;
    overflow: hidden;
  }
  .summary .sr {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 14px;
    background: var(--raised);
  }
  .summary .sk {
    font-size: 12px;
    color: var(--text-muted);
    width: 96px;
    flex: none;
  }
  .summary .sv {
    font-size: 12.5px;
    font-weight: 560;
    color: var(--text-primary);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .summary .mono {
    font-family: ui-monospace, "SF Mono", Menlo, monospace;
    font-size: 11.5px;
  }

  /* ---- footer ---- */
  .wiz-foot {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 26px;
    border-top: 0.5px solid var(--gridline);
    background: var(--surface-solid);
  }
  .wiz-foot .grow {
    flex: 1;
  }
  .wiz-foot .wide {
    min-width: 180px;
  }
</style>
