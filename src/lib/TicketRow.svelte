<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Issue } from "../types";
  import { t } from "../i18n.svelte";

  let {
    issue,
    showAge = false,
    color = null,
  }: {
    issue: Issue;
    showAge?: boolean;
    color?: string | null;
  } = $props();

  // Hien TEN GOI chu khong phai chu viet tat: "TN" bat nguoi doc phai giai ma,
  // "Tuan" thi nhan ra ngay. `shortName` cua ticket chua giao do Rust dat san
  // bang tieng Viet — thay bang chuoi i18n de doi ngon ngu van dung.
  let who = $derived(issue.assignee ? issue.shortName : t("unassigned"));

  async function open() {
    try {
      await invoke("open_issue", { url: issue.url });
    } catch (e) {
      console.error("khong mo duoc ticket", e);
    }
  }
</script>

<button class="row" onclick={open} title={issue.summary}>
  <span class="key num">{issue.key}</span>

  <!-- "Open · Chien": status truoc, nguoi sau — mot o co gian duy nhat -->
  <span class="st" title={issue.assigneeDisplay ?? t("unassigned")}>
    {issue.status}
    <span class="who">
      · {#if color}<span class="who-dot" style:background={color}></span>{/if}{who}
    </span>
  </span>

  {#if showAge}
    <span class="metric num" class:warn={issue.isOld} title={t("ageTitle")}>
      {#if issue.isOld}<span class="dot dot-warning"></span>{/if}{issue.ageDays}d
    </span>
  {/if}

  <span class="metric num" class:crit={issue.isStale} title={t("idleTitle")}>
    {#if issue.isStale}<span class="dot dot-critical"></span>{/if}{issue.idleDays}d
  </span>
</button>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--sp-lg);
    width: 100%;
    padding: 4px;
    border-radius: 5px;
    font-size: 11px;
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--raised);
  }

  .key {
    font-weight: 620;
    color: var(--text-primary);
    flex: none;
    width: 72px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .st {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
  }
  .who {
    color: var(--text-secondary);
  }
  .who-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    margin-right: 2px;
    vertical-align: 0;
  }

  /* Chu luon deo token chu; cham mau chi la dau hieu phu di kem con so. */
  .metric {
    flex: none;
    min-width: 34px;
    text-align: right;
    color: var(--text-muted);
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 3px;
  }
  .metric.warn {
    color: var(--text-primary);
    font-weight: 640;
  }
  .metric.crit {
    color: var(--status-critical);
    font-weight: 640;
  }
  .dot {
    width: 5px;
    height: 5px;
  }
</style>
