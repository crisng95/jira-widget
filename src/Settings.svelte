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
    notify: Notify;
    hasToken: boolean;
  };
  type Board = { id: number; name: string; boardType: string };
  type Msg = { kind: "ok" | "err" | "warn"; text: string } | null;

  const TABS = [
    { id: "conn", label: "Kết nối" },
    { id: "scope", label: "Phạm vi" },
    { id: "alert", label: "Cảnh báo" },
    { id: "view", label: "Hiển thị" },
  ] as const;

  let tab = $state<(typeof TABS)[number]["id"]>("conn");
  let cfg = $state<Dto | null>(null);
  let boards = $state<Board[]>([]);
  let token = $state("");
  let msg = $state<Msg>(null);
  let busy = $state("");
  let testText = $state("");
  let reviewText = $state("");
  let releaseText = $state("");
  // Danh sach status THAT cua project. Rong => chua tai duoc, rot ve go tay.
  let allStatuses = $state<string[]>([]);

  // onMount PHAI dong bo: callback async tra ve Promise, Svelte khong coi do
  // la ham don dep nen listener se khong bao gio duoc go.
  onMount(() => {
    // Mode đổi được từ menu bar hoặc chip trên panel trong lúc cửa sổ này đang
    // mở. Không nghe thì radio hiện một đằng, panel chạy một nẻo — rồi bấm Lưu
    // là ghi đè ngược lại cái vừa chọn.
    const un = listen<string>("panel://display-mode", (ev) => {
      if (cfg) cfg.displayMode = ev.payload;
      modeDaAp = ev.payload;
    });

    void (async () => {
      try {
        const d = await invoke<Dto>("settings_get");
        cfg = d;
        modeDaAp = d.displayMode;
        testText = d.testStatuses.join("\n");
        reviewText = d.reviewStatuses.join("\n");
        releaseText = d.pendingReleaseStatuses.join("\n");
        if (!d.hasToken) {
          msg = {
            kind: "warn",
            text: "Chưa có token. Dán Personal Access Token vào ô bên dưới.",
          };
        }
        void loadBoards(true);
        void loadStatuses(true);
      } catch (e) {
        msg = { kind: "err", text: `Không đọc được cài đặt: ${e}` };
      }
    })();

    return () => {
      un.then((f) => f());
    };
  });

  // Mode KHONG di qua nut Luu. No la thu duy nhat doi nong duoc, nen bam la ap
  // dung luon — bat nguoi ta bam them "Luu & khoi dong lai" cho mot thu von
  // khong can khoi dong lai thi vua thua vua sai.
  // Mode DA duoc backend chap nhan. Phai giu rieng: khong doc nguoc `cfg` de
  // roll back duoc, vi `bind:group` la listener gan THANG vao input con
  // `onchange` thi duoc uy quyen len root — bind chay truoc, nen luc vao day
  // `cfg.displayMode` DA la gia tri vua bam. Doc no ra lam "gia tri cu" thi
  // nhanh catch thanh lenh khong.
  let modeDaAp = $state("team");

  async function doiMode(m: string) {
    if (!cfg) return;
    try {
      // Rust tra ve mode THUC SU duoc ap dung — no co the tu ha ve "team".
      const thuc = await invoke<string>("set_display_mode", { mode: m });
      cfg.displayMode = thuc;
      modeDaAp = thuc;
    } catch (e) {
      cfg.displayMode = modeDaAp; // radio phai quay ve dung cai panel dang chay
      msg = { kind: "err", text: `Không đổi được chế độ: ${e}` };
    }
  }

  async function loadBoards(quiet = false) {
    if (!cfg) return;
    busy = "boards";
    try {
      boards = await invoke<Board[]>("settings_list_boards", {
        jiraUrl: cfg.jiraUrl,
        projectKey: cfg.projectKey,
        tokenOverride: token.trim() || null,
      });
      if (!quiet) {
        msg = { kind: "ok", text: `Tìm thấy ${boards.length} board của ${cfg.projectKey}.` };
      }
    } catch (e) {
      boards = [];
      if (!quiet) msg = { kind: "err", text: `Không lấy được danh sách board: ${e}` };
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
        tokenOverride: token.trim() || null,
      });
      if (!quiet) {
        msg = { kind: "ok", text: `Tìm thấy ${allStatuses.length} status của ${cfg.projectKey}.` };
      }
    } catch (e) {
      allStatuses = [];
      if (!quiet) msg = { kind: "err", text: `Không lấy được danh sách status: ${e}` };
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
        { jiraUrl: cfg.jiraUrl, tokenOverride: token.trim() || null },
      );
      // Tien the dien luon username vao o `me` — khoi phai di tra cuu
      const filled = !cfg.me.trim() && !!who.name;
      if (filled) cfg.me = who.name;
      msg = {
        kind: "ok",
        text: `Kết nối OK — ${who.displayName} (${who.name})${filled ? " · đã điền vào Username" : ""}`,
      };
    } catch (e) {
      msg = { kind: "err", text: `Kết nối thất bại: ${e}` };
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
      msg = { kind: "ok", text: "Đã lưu token vào Keychain." };
    } catch (e) {
      msg = { kind: "err", text: `Không lưu được token: ${e}` };
    } finally {
      busy = "";
    }
  }

  async function clearToken() {
    busy = "token";
    try {
      await invoke("settings_clear_token");
      if (cfg) cfg.hasToken = false;
      msg = {
        kind: "warn",
        text: "Đã xoá token. Panel sẽ không lấy được dữ liệu cho tới khi có token mới.",
      };
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
        msg = { kind: "ok", text: "Đã lưu. Bấm “Lưu & khởi động lại” để áp dụng toàn bộ." };
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
    <h1>Cài đặt</h1>
    <div class="tabs" role="tablist">
      {#each TABS as t (t.id)}
        <button role="tab" aria-selected={tab === t.id} onclick={() => (tab = t.id)}>
          {t.label}
        </button>
      {/each}
    </div>
  </div>

  <div class="sw-body">
    {#if msg}
      <div class="msg {msg.kind}"><span class="dot"></span><span>{msg.text}</span></div>
    {/if}

    {#if !cfg}
      <p class="muted">Đang tải…</p>
    {:else if tab === "conn"}
      <div class="field">
        <label for="url">Jira URL</label>
        <input id="url" type="text" bind:value={cfg.jiraUrl} placeholder="https://jira.cong-ty.vn" />
      </div>

      <div class="field top">
        <label for="tok">Access token</label>
        <div class="stack">
          <div class="row">
            <input
              id="tok"
              type="password"
              bind:value={token}
              placeholder={cfg.hasToken ? "•••••• dán token mới để thay" : "dán Personal Access Token"}
              autocomplete="off"
            />
            <button onclick={saveToken} disabled={busy !== "" || !token.trim()}>Lưu</button>
          </div>
          <!-- Trang thai token va nut xoa nam o DONG RIENG, khong nhet giua doan van -->
          <div class="tokline">
            {#if cfg.hasToken}
              <span class="ok">✓ đã lưu trong Keychain</span>
              <button class="link" onclick={clearToken} disabled={busy !== ""}>xoá</button>
            {:else}
              <span>chưa có token</span>
            {/if}
          </div>
        </div>
      </div>
      <p class="hint">Jira → avatar góc phải → Profile → Personal Access Tokens → Create token.</p>

      <div class="field">
        <span class="lbl"></span>
        <button onclick={testConnection} disabled={busy !== ""} style="justify-self:start">
          {busy === "test" ? "Đang kiểm tra…" : "Kiểm tra kết nối"}
        </button>
      </div>
      <p class="hint">Xác thực token và tự điền username của bạn.</p>
    {:else if tab === "scope"}
      <div class="field">
        <label for="pk">Project key</label>
        <div class="row">
          <input id="pk" type="text" bind:value={cfg.projectKey} placeholder="PROJ" />
          <button
            onclick={() => {
              void loadBoards();
              void loadStatuses(true);
            }}
            disabled={busy !== ""}
          >
            {busy === "boards" ? "Đang tìm…" : "Tìm board"}
          </button>
        </div>
      </div>

      <div class="field">
        <label for="bd">Board</label>
        {#if boards.length > 0}
          <select id="bd" bind:value={cfg.boardId}>
            {#each boards as b (b.id)}
              <option value={b.id}>{b.name} · {b.boardType} · id {b.id}</option>
            {/each}
          </select>
        {:else}
          <div class="num">
            <input id="bd" type="number" bind:value={cfg.boardId} min="1" />
            <span class="unit">bấm “Tìm board” để chọn từ danh sách</span>
          </div>
        {/if}
      </div>
      <p class="hint">Luôn lấy sprint đang chạy của board này, tự bám theo khi sang sprint mới.</p>

      <div class="field">
        <label for="me">Username</label>
        <input id="me" type="text" bind:value={cfg.me} placeholder="vd: sam.hale" />
      </div>
      <p class="hint">Tô đậm dòng của bạn, và hiện mục “Chờ tôi duyệt”. Để trống thì tắt.</p>

      <div class="sep"></div>
      <p class="group-title">Hàng đợi</p>
      <p class="hint qhint">
        Status quyết định “đang chờ gì”, field người quyết định “chờ ai”.
        Chờ test lọc theo <b>QCs</b>, chờ duyệt lọc theo <b>Approvers</b> —
        hai vai trò khác nhau. Chờ release không lọc theo người.
      </p>

      {#each [{ id: "test", label: "Chờ test", show: "showTestQueue" }, { id: "review", label: "Chờ duyệt", show: "showReviewQueue" }, { id: "release", label: "Chờ release", show: "showReleaseQueue" }] as q (q.id)}
        <div class="qbox">
          <label class="qhead">
            <input
              type="checkbox"
              checked={cfg[q.show as "showTestQueue"]}
              onchange={(e) =>
                cfg && ((cfg as any)[q.show] = (e.currentTarget as HTMLInputElement).checked)}
            />
            <span class="qname">{q.label}</span>
            <span class="qcount">{textOf(q.id).split("\n").filter((x) => x.trim()).length} status</span>
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
                Chưa tải được danh sách status —
                <button class="link" onclick={() => loadStatuses()} disabled={busy !== ""}>
                  thử lại</button
                >. Gõ sai một chữ là lọc ra 0 ticket.
              </p>
            {/if}
          {/if}
        </div>
      {/each}

    {:else if tab === "alert"}
      <div class="field">
        <label for="poll">Chu kỳ poll</label>
        <div class="num">
          <input id="poll" type="number" bind:value={cfg.pollIntervalSecs} min="10" />
          <span class="unit">giây</span>
        </div>
      </div>

      <div class="field">
        <label for="stale">Đứng im quá</label>
        <div class="num">
          <input id="stale" type="number" bind:value={cfg.staleDays} min="0" />
          <span class="unit">ngày → cảnh báo đỏ</span>
        </div>
      </div>

      <div class="field">
        <label for="soon">Sắp hết sprint</label>
        <div class="num">
          <input id="soon" type="number" bind:value={cfg.endingSoonHours} min="0" />
          <span class="unit">giờ cuối</span>
        </div>
      </div>

      <div class="field">
        <label for="old">Ticket già</label>
        <div class="num">
          <input id="old" type="number" bind:value={cfg.oldAgeDays} min="0" />
          <span class="unit">ngày tuổi</span>
        </div>
      </div>

      <div class="sep"></div>
      <p class="group-title">Thông báo</p>

      <div class="field top">
        <span class="lbl">Báo khi</span>
        <div class="checks">
          <label><input type="checkbox" bind:checked={cfg.notify.statusChanged} /> đổi status</label>
          <label><input type="checkbox" bind:checked={cfg.notify.assigneeChanged} /> đổi người làm</label>
          <label><input type="checkbox" bind:checked={cfg.notify.added} /> ticket mới</label>
          <label><input type="checkbox" bind:checked={cfg.notify.removed} /> rời sprint</label>
        </div>
      </div>

      <div class="field">
        <label for="gt">Gộp khi quá</label>
        <div class="num">
          <input id="gt" type="number" bind:value={cfg.notify.groupThreshold} min="1" />
          <span class="unit">thay đổi một lúc</span>
        </div>
      </div>
    {:else}
      <div class="field top">
        <span class="lbl">Panel hiện</span>
        <div class="stack">
          <label class="row">
            <input
              type="radio"
              bind:group={cfg.displayMode}
              value="team"
              onchange={() => doiMode("team")}
            />
            <span>Cả team — toàn bộ sprint, mọi người</span>
          </label>
          <label class="row">
            <input
              type="radio"
              bind:group={cfg.displayMode}
              value="only_me"
              disabled={!cfg.me.trim()}
              onchange={() => doiMode("only_me")}
            />
            <span class:disabled={!cfg.me.trim()}>
              Chỉ việc của tôi — lọc theo username của bạn
            </span>
          </label>
        </div>
      </div>
      <p class="hint">
        {#if !cfg.me.trim()}
          Cần điền <b>Username</b> ở tab Kết nối trước — không biết “tôi” là ai thì
          không lọc được.
        {:else}
          Áp dụng ngay khi bấm, không cần Lưu và không cần khởi động lại. Đổi được ở
          cả ba chỗ: đây, mục “Chỉ việc của tôi” trên menu bar, và chip trên panel.
          Hàng đợi chờ test / duyệt / release vẫn giữ nguyên vì chúng lọc theo vai
          trò, không theo người làm.
        {/if}
      </p>

      <div class="field top">
        <span class="lbl">Panel nằm ở</span>
        <div class="stack">
          <label class="row">
            <input type="radio" bind:group={cfg.windowLayer} value="desktop" />
            <span>Dán vào desktop — không bao giờ che app khác</span>
          </label>
          <label class="row">
            <input type="radio" bind:group={cfg.windowLayer} value="floating" />
            <span>Nổi trên tất cả — luôn nhìn thấy</span>
          </label>
        </div>
      </div>
      <p class="hint">
        Ở tầng desktop, vài máy macOS không gửi được click tới panel. Bấm ticket mà không
        mở được browser thì đổi sang “Nổi trên tất cả”.
      </p>
    {/if}
  </div>

  <div class="sw-foot">
    <button class="primary" onclick={() => save(true)} disabled={busy !== ""}>
      {busy === "save" ? "Đang lưu…" : "Lưu & khởi động lại"}
    </button>
    <button onclick={() => save(false)} disabled={busy !== ""}>Chỉ lưu</button>
    <span class="spacer"></span>
    <button onclick={() => invoke("settings_close")}>Đóng</button>
  </div>
</div>
