<script lang="ts">
  // Luu y: KHONG duoc dat bien ten `state` — Svelte 5 se hieu `$state` la
  // store subscription chu khong phai rune. Dung `panel`.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { PanelState } from "./types";
  import { colorMap } from "./types";
  import { t, setLang } from "./i18n.svelte";

  import Header from "./lib/Header.svelte";
  import RiskAlerts from "./lib/RiskAlerts.svelte";
  import SprintProgress from "./lib/SprintProgress.svelte";
  import Allocation from "./lib/Allocation.svelte";
  import MemberLoadList from "./lib/MemberLoadList.svelte";
  import QueueSection from "./lib/QueueSection.svelte";
  import TicketRow from "./lib/TicketRow.svelte";

  let panel = $state<PanelState | null>(null);
  let compact = $state(false);
  let showAll = $state(false);
  let moving = $state(false);

  let snap = $derived(panel?.snapshot ?? null);
  let colors = $derived(
    snap ? colorMap(snap.colorOrder) : new Map<string, string>(),
  );

  let onlyMe = $derived(snap?.displayMode === "only_me");
  // Hai kieu "rong" khac han nhau ve y nghia nen phai noi hai cau khac nhau:
  // khong duoc giao viec != da lam xong het viec.
  let khongCoViec = $derived(onlyMe && snap !== null && snap.issues.length === 0);
  let xongHetViec = $derived(
    onlyMe && snap !== null && snap.issues.length > 0 && snap.openIssues.length === 0,
  );

  async function veTeam() {
    try {
      await invoke("set_display_mode", { mode: "team" });
    } catch (e) {
      console.error("set_display_mode loi", e);
    }
  }

  async function moCaiDat() {
    try {
      await invoke("settings_open");
    } catch (e) {
      console.error("settings_open loi", e);
    }
  }

  async function expand() {
    compact = false;
    await invoke("set_compact", { compact: false });
  }

  // Che do di chuyen: panel tam noi len tren de nhan chuot; keo tu do bang
  // vung drag cua banner; bam Xong la tra ve dung tang da cau hinh.
  async function batDiChuyen() {
    if (moving) return;
    moving = true;
    try {
      await invoke("set_move_mode", { moving: true });
    } catch (e) {
      console.error("set_move_mode loi", e);
    }
  }

  async function xongDiChuyen() {
    moving = false;
    try {
      await invoke("set_move_mode", { moving: false });
    } catch (e) {
      console.error("set_move_mode loi", e);
    }
  }

  onMount(() => {
    invoke<PanelState>("get_state")
      .then((s) => {
        panel = s;
        setLang(s.language);
      })
      .catch((e) => console.error("get_state loi", e));

    const un = listen<PanelState>("panel://state", (ev) => {
      panel = ev.payload;
      setLang(ev.payload.language);
    });

    // Tray bam "Thu gon / mo rong" -> lat o day de trang thai chi co MOT nguon
    const unCompact = listen("panel://toggle-compact", () => {
      compact = !compact;
      invoke("set_compact", { compact }).catch((e) =>
        console.error("set_compact loi", e),
      );
    });

    const unLang = listen<string>("panel://language", (ev) => {
      setLang(ev.payload);
    });

    const unMove = listen("panel://move-mode", () => {
      void batDiChuyen();
    });

    return () => {
      un.then((f) => f());
      unCompact.then((f) => f());
      unLang.then((f) => f());
      unMove.then((f) => f());
    };
  });
</script>

<div class="panel" class:moving>
  {#if moving}
    <div class="move-banner" data-tauri-drag-region>
      <span class="grip" data-tauri-drag-region>✥</span>
      <span class="mt" data-tauri-drag-region>{t("moveHint")}</span>
      <button class="done2" onclick={xongDiChuyen}>{t("done")}</button>
    </div>
  {/if}

  {#if panel === null}
    <div class="boot">{t("loading")}</div>
  {:else}
    <Header {panel} bind:compact />

    {#if !compact}
      <div class="scroll">
        {#if panel.onboarding && !snap}
          <!-- Chua cai dat xong: chao mung + loi ra, KHONG doa nguoi dung bang
               loi ket noi do choi — ho con chua nhap gi ca. -->
          <div class="section notice">
            <p class="ntitle">
              <span class="dot dot-mine"></span> {t("welcomeTitle")}
            </p>
            <p class="nbody muted">{t("welcomeBody")}</p>
            <button class="back" onclick={() => invoke("onboarding_open")}>
              {t("openGuide")}
            </button>
          </div>
        {:else if panel.errorKind === "auth"}
          <!-- Loi auth khong duoc im lang: panel trang thi anh tuong sprint rong.
               "Token het han" phai co loi ra ngay tai cho: nut mo Cai dat. -->
          <div class="section notice">
            <p class="ntitle"><span class="dot dot-critical"></span> {t("tokenExpired")}</p>
            <p class="nbody">{panel.errorMessage}</p>
            <p class="nbody muted">{t("tokenExpiredHint")}</p>
            <button class="back" onclick={moCaiDat}>{t("openSettings")}</button>
          </div>
        {:else if !panel.ok && !snap}
          <div class="section notice">
            <p class="ntitle">
              <span class="dot dot-critical"></span> {t("cantReachJira")}
            </p>
            <p class="nbody">{panel.errorMessage}</p>
            <p class="nbody muted num">{t("triedTimes", { n: panel.consecutiveFailures })}</p>
          </div>
        {:else if panel.noActiveSprint}
          <div class="section notice">
            <p class="ntitle">
              <span class="dot dot-neutral"></span> {t("noSprint")}
            </p>
            <p class="nbody muted">{t("noSprintBody")}</p>
          </div>
        {/if}

        {#if snap}
          {#if !panel.ok}
            <!-- Van hien du lieu cu, chi ghi ro no la du lieu cu -->
            <div class="stalebar">
              <span class="dot dot-warning"></span>
              {t("staleBar", { n: panel.consecutiveFailures })}
            </div>
          {/if}

          <RiskAlerts risks={snap.risks} staleDays={panel.staleDays} {colors} />
          {#if khongCoViec}
            <!-- Man hinh rong PHAI tu giai thich. Bat Only Me roi thay panel
                 trang la luc nguoi dung tuong app hong, trong khi su that chi
                 la sprint nay chua giao viec cho ho. -->
            <div class="section notice">
              <p class="ntitle">
                <span class="dot dot-neutral"></span> {t("noTicketsMine")}
              </p>
              <p class="nbody muted num">
                {t("sprintOverall", {
                  d: snap.sprintContext.done,
                  t: snap.sprintContext.total,
                  p: snap.sprintContext.percent,
                })}
              </p>
              <button class="back" onclick={veTeam}>{t("viewTeam")}</button>
            </div>
          {:else}
            <SprintProgress
              progress={snap.progress}
              points={snap.points}
              sprintContext={snap.sprintContext}
              {onlyMe}
            />
            <!-- Donut phan bo va bang tai member la de so sanh GIUA nguoi voi
                 nguoi. Con dung mot nguoi thi donut luon 100% mot mau va bang
                 tai chi con mot dong — hai o vo nghia chiem cho. -->
            {#if !onlyMe}
              <Allocation members={snap.byAssignee} {colors} />
              <MemberLoadList members={snap.byAssignee} {colors} />
            {/if}
          {/if}
          <QueueSection
            queue={snap.testQueue}
            titleMine={t("qTestMine")}
            titleAll={t("qTestAll")}
            emptyMine={t("qTestEmptyMine")}
            emptyAll={t("qTestEmptyAll")}
            {colors}
          />
          <QueueSection
            queue={snap.reviewQueue}
            titleMine={t("qReviewMine")}
            titleAll={t("qReviewAll")}
            emptyMine={t("qReviewEmptyMine")}
            emptyAll={t("qReviewEmptyAll")}
            {colors}
          />
          <!-- Chờ release không lọc theo người ở BẤT KỲ mode nào. Ở Only Me mọi
               thứ khác trên panel đều đã lọc, nên tiêu đề trần "Chờ release" sẽ
               bị đọc thành "của tôi" — phải tự nói ra là cả team. -->
          <QueueSection
            queue={snap.releaseQueue}
            titleMine={t("qRelease")}
            titleAll={onlyMe ? t("qReleaseTeam") : t("qRelease")}
            emptyMine={t("qReleaseEmpty")}
            emptyAll={onlyMe ? t("qReleaseEmptyTeam") : t("qReleaseEmpty")}
            {colors}
          />

          {#if !khongCoViec}
            <div class="section">
              <div class="section-head">
                <button class="alltoggle" onclick={() => (showAll = !showAll)}>
                  <span class="section-title">
                    {onlyMe ? t("myTickets") : t("allTickets")}
                  </span>
                  <span class="caret">{showAll ? "▾" : "▸"}</span>
                </button>
                <span class="count-pill num" title={t("ageStatsTitle")}>
                  {t("nOpen", { n: snap.openIssues.length })}
                </span>
              </div>
              {#if xongHetViec}
                <!-- Khong gap sau caret: xong het viec la tin tot, dang duoc
                     noi thang chu khong bat nguoi ta bam ra moi thay. -->
                <p class="doneall">{t("allDoneMine")}</p>
              {:else if showAll}
                <p class="agestat num muted">
                  {t("ageStats", { p: snap.ageStats.medianAge, m: snap.ageStats.maxAge })}
                </p>
                {#each snap.openIssues as issue (issue.key)}
                  <TicketRow
                    {issue}
                    showAge={true}
                    color={issue.assignee ? colors.get(issue.assignee) : null}
                  />
                {/each}
              {/if}
            </div>
          {/if}
        {/if}
      </div>
    {:else if snap}
      <!-- Thu gon: chi tien do + so canh bao + nut mo rong -->
      <div class="cstrip">
        <span class="cnum num">{snap.progress.done}/{snap.progress.total}</span>
        <span class="ctrack">
          <span class="cfill" style:width="{snap.progress.percent}%"></span>
        </span>
        <span class="cwarn num" class:hot={snap.risks.count > 0}>
          {#if snap.risks.count > 0}<span class="dot dot-critical"></span>{/if}
          {snap.risks.count}
        </span>
        <button class="cexpand" onclick={expand} title={t("expandTitle")}>▾</button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .boot {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--text-muted);
    font-size: 11px;
  }

  /* --- trang thai rong / loi: khong bao gio man trang, luon co loi ra --- */
  .notice {
    padding: 12px var(--pad-x);
  }
  .notice .ntitle {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0 0 5px;
    font-size: 11.5px;
    font-weight: 600;
  }
  .notice .nbody {
    margin: 0 0 3px;
    font-size: 10.5px;
    color: var(--text-secondary);
    word-break: break-word;
  }
  .back {
    margin-top: 9px;
    padding: 4px 11px;
    border-radius: var(--r-md);
    background: var(--raised);
    color: var(--text-primary);
    font-size: 10.5px;
    font-weight: 640;
    cursor: pointer;
  }
  .back:hover {
    background: var(--border);
  }

  .dot-mine {
    background: var(--series-1);
  }

  .stalebar {
    display: flex;
    align-items: center;
    gap: var(--sp-md);
    padding: 5px var(--pad-x);
    font-size: 10.5px;
    color: var(--text-secondary);
    background: var(--raised);
  }

  .alltoggle {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    cursor: pointer;
  }
  .caret {
    color: var(--text-muted);
    font-size: 9px;
  }

  .agestat {
    margin: -4px 0 5px;
    font-size: 9.5px;
  }

  .doneall {
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--status-good);
  }

  /* --- thu gon --- */
  .cstrip {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 var(--pad-x) 9px;
  }
  .cnum {
    font-size: 11px;
    font-weight: 640;
  }
  .ctrack {
    flex: 1;
    height: 8px;
    background: var(--raised);
    border-radius: var(--r-sm);
    overflow: hidden;
  }
  .cfill {
    display: block;
    height: 100%;
    background: var(--stage-closed);
    border-radius: 0 var(--r-sm) var(--r-sm) 0;
  }
  .cwarn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .cwarn.hot {
    color: var(--text-primary);
    font-weight: 640;
  }
  .cexpand {
    width: 21px;
    height: 19px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    color: var(--text-muted);
    font-size: 12px;
    cursor: pointer;
  }
  .cexpand:hover {
    background: var(--raised);
    color: var(--text-primary);
  }

  /* --- che do di chuyen --- */
  .panel.moving {
    outline: 2px solid var(--series-1);
    outline-offset: -2px;
  }
  .move-banner {
    position: absolute;
    inset: 0;
    z-index: 5;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 11px;
    background: var(--surface);
    backdrop-filter: blur(3px);
    -webkit-backdrop-filter: blur(3px);
    text-align: center;
  }
  .move-banner .grip {
    font-size: 22px;
  }
  .move-banner .mt {
    font-size: 12px;
    font-weight: 640;
    color: var(--text-primary);
    max-width: 220px;
  }
  .move-banner .done2 {
    font-size: 11px;
    font-weight: 640;
    padding: 5px 15px;
    border-radius: 7px;
    background: var(--series-1);
    color: #fff;
    cursor: pointer;
  }
</style>
