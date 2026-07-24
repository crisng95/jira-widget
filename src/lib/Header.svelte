<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { PanelState } from "../types";
  import { fmtCountdown, fmtClock } from "../types";

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
    <span class="cd" class:critical={urgency === "critical"} class:warning={urgency === "warning"}>
      {#if urgency !== "none"}<span
          class="dot"
          class:dot-critical={urgency === "critical"}
          class:dot-warning={urgency === "warning"}
        ></span>{/if}
      {fmtCountdown(secs)}
    </span>
  </div>

  <div class="line2" data-tauri-drag-region>
    <span class="stamp">
      {#if !panel.ok}
        <span class="dot dot-critical"></span>
        <span class="crit-text">
          {panel.errorKind === "auth" ? "Token het han" : "Du lieu luc"}
          {panel.errorKind === "auth" ? "" : fmtClock(panel.lastSuccess)}
        </span>
      {:else if panel.noActiveSprint}
        <span class="dot dot-neutral"></span> Chua co sprint dang chay
      {:else}
        <span class="dot dot-good"></span> cap nhat {fmtClock(panel.lastSuccess)}
      {/if}
    </span>

    <span class="actions">
      {#if onlyMe}
        <button
          class="modechip"
          onclick={veTeam}
          title="Đang lọc theo bạn — bấm để xem lại cả team"
        >
          <span class="dot dot-mine"></span>
          {viewer?.short ?? "tôi"} · chỉ việc của tôi
        </button>
      {/if}
      <button onclick={refresh} title="Refresh ngay" class:spin={busy}>&#8635;</button>
      <button onclick={toggleCompact} title={compact ? "Mo rong" : "Thu gon"}>
        {compact ? "▾" : "▴"}
      </button>
    </span>
  </div>
</header>

<style>
  header {
    padding: 9px 12px 8px;
    flex: none;
    cursor: default;
  }

  .line1 {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }

  .title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cd {
    flex: none;
    font-size: 12px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .cd.warning,
  .cd.critical {
    color: var(--text-primary);
  }

  .line2 {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 2px;
    font-size: 10px;
    color: var(--text-muted);
  }

  /* Panel rong co dinh 360px. Chip mode la `nowrap`, nen khi ten dai + dau thoi
     gian + hai nut icon cong lai se day nhau tran ra ngoai. Cho dau thoi gian
     co lai truoc, vi no la thu it quan trong nhat trong ba. */
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

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex: none;
  }
  .dot-good {
    background: var(--status-good);
  }
  .dot-warning {
    background: var(--status-warning);
  }
  .dot-critical {
    background: var(--status-critical);
  }
  .dot-neutral {
    background: var(--baseline);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .actions button {
    width: 20px;
    height: 18px;
    border-radius: 4px;
    color: var(--text-muted);
    font-size: 12px;
    cursor: pointer;
  }

  /* Chip nay khong phai nut bam thong thuong: no la nhan trang thai "man hinh
     dang bi loc". Vi the no rong ra theo chu va dam hon cac nut icon ben canh. */
  .actions .modechip {
    width: auto;
    height: 18px;
    padding: 0 7px;
    margin-right: 4px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    font-weight: 600;
    white-space: nowrap;
    color: var(--text-secondary);
    background: var(--raised);
  }
  .actions .modechip:hover {
    color: var(--text-primary);
  }
  .dot-mine {
    background: var(--series-1);
  }
  .actions button:hover {
    background: var(--raised);
    color: var(--text-primary);
  }
  .actions button.spin {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
