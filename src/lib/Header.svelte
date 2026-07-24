<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { PanelState } from "../types";
  import { t, fmtCountdown, fmtClock } from "../i18n.svelte";

  // Prop ten `panel` chu khong phai `state`: dat ten `state` se lam Svelte 5
  // hieu `$state` la store subscription thay vi rune.
  let {
    panel,
    compact = $bindable(false),
  }: { panel: PanelState; compact: boolean } = $props();

  let busy = $state(false);

  // Nhip 1 giay chi de dem nguoc chay muot. Poll van la 60s — cai nay
  // khong goi mang, chi tinh lai tu sprintEnd da co san.
  let tick = $state(Date.now());
  $effect(() => {
    const id = setInterval(() => (tick = Date.now()), 1000);
    return () => clearInterval(id);
  });

  let secs = $derived.by(() => {
    const end = panel.snapshot?.sprintEnd;
    if (end) return Math.round((new Date(end).getTime() - tick) / 1000);
    return panel.snapshot?.secondsLeft ?? null;
  });
  // Do gap: < 4h la do, < 24h la vang. Mau di kem CHU dem nguoc nen
  // khong bao gio phai doc bang mau don thuan.
  let urgency = $derived.by(() => {
    if (secs === null) return "none";
    if (secs < 0) return "critical";
    if (secs < 4 * 3600) return "critical";
    if (secs < 24 * 3600) return "warning";
    return "none";
  });

  async function refresh() {
    busy = true;
    try {
      await invoke("refresh_now");
    } finally {
      setTimeout(() => (busy = false), 1200);
    }
  }

  async function toggleCompact() {
    compact = !compact;
    await invoke("set_compact", { compact });
  }

  async function hide() {
    try {
      await invoke("hide_panel");
    } catch (e) {
      console.error("hide_panel loi", e);
    }
  }

  // Chip chi hien khi dang o Only Me, va bam vao la ve Team.
  //
  // Ly do phai luon hien: doi mode xong roi quen la moi nguon hieu nham nang
  // nhat — "sao ca team chi con 4 ticket". Mot mode dang bo bot du lieu thi
  // phai tu noi ra, khong the nam im trong menu tray.
  let onlyMe = $derived(panel.snapshot?.displayMode === "only_me");
  let viewer = $derived(panel.snapshot?.viewer ?? null);

  async function veTeam() {
    try {
      await invoke("set_display_mode", { mode: "team" });
    } catch (e) {
      console.error("set_display_mode loi", e);
    }
  }
</script>

<header data-tauri-drag-region>
  <div class="line1" data-tauri-drag-region>
    <span class="title" data-tauri-drag-region>
      {panel.snapshot?.sprintName ?? "PROJ"}
    </span>
    <span class="cd num" class:critical={urgency === "critical"} class:warning={urgency === "warning"}>
      {#if urgency !== "none"}<span
          class="dot"
          class:dot-critical={urgency === "critical"}
          class:dot-warning={urgency === "warning"}
        ></span>{/if}
      {fmtCountdown(secs)}
    </span>
  </div>

  {#if !compact}
    <div class="line2" data-tauri-drag-region>
      <span class="stamp">
        {#if !panel.ok}
          <span class="dot dot-critical"></span>
          <span class="crit-text">
            {panel.errorKind === "auth"
              ? t("tokenExpired")
              : t("dataFrom", { t: fmtClock(panel.lastSuccess) })}
          </span>
        {:else if panel.noActiveSprint}
          <span class="dot dot-neutral"></span> {t("noSprint")}
        {:else}
          <span class="dot dot-good"></span> {t("updatedAt", { t: fmtClock(panel.lastSuccess) })}
        {/if}
      </span>

      <span class="actions">
        {#if onlyMe}
          <button class="modechip" onclick={veTeam} title={t("onlyMeChipTitle")}>
            <span class="dot dot-mine"></span>
            {t("onlyMeChip", { name: viewer?.short || t("meFallback") })}
          </button>
        {/if}
        <button class="ico" onclick={refresh} title={t("refreshTitle")} class:spin={busy}>&#8635;</button>
        <button class="ico" onclick={toggleCompact} title={t("collapseTitle")}>▴</button>
        <button class="ico" onclick={hide} title={t("hideTitle")}>⤓</button>
      </span>
    </div>
  {/if}
</header>

<style>
  header {
    padding: var(--pad-y) var(--pad-x) 9px;
    flex: none;
    cursor: default;
  }

  .line1 {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--sp-lg);
  }

  .title {
    font-size: 13px;
    font-weight: 640;
    letter-spacing: -0.01em;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd {
    flex: none;
    font-size: 12px;
    font-weight: 640;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .cd.warning,
  .cd.critical {
    color: var(--text-primary);
  }

  .line2 {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 3px;
    font-size: 10.5px;
    color: var(--text-muted);
  }

  /* Panel rong co dinh 360px. Chip mode la `nowrap`, nen khi ten dai + dau thoi
     gian + cac nut icon cong lai se day nhau tran ra ngoai. Cho dau thoi gian
     co lai truoc, vi no la thu it quan trong nhat. */
  .stamp {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .crit-text {
    color: var(--text-secondary);
  }

  .dot-mine {
    background: var(--series-1);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 3px;
  }
  .actions .ico {
    width: 21px;
    height: 19px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    color: var(--text-muted);
    font-size: 12px;
    cursor: pointer;
  }
  .actions .ico:hover {
    background: var(--raised);
    color: var(--text-primary);
  }

  /* Chip nay khong phai nut bam thong thuong: no la nhan trang thai "man hinh
     dang bi loc". Vi the no rong ra theo chu va dam hon cac nut icon ben canh. */
  .actions .modechip {
    height: 19px;
    padding: 0 8px;
    margin-right: 3px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    border-radius: var(--r-full);
    font-size: 10px;
    font-weight: 640;
    white-space: nowrap;
    color: var(--text-secondary);
    background: var(--raised);
    cursor: pointer;
  }
  .actions .modechip:hover {
    color: var(--text-primary);
  }
  .actions .ico.spin {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
