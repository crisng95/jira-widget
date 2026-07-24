<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Issue } from "../types";

  let {
    issue,
    showStatus = true,
    showAge = false,
    color = null,
  }: {
    issue: Issue;
    showStatus?: boolean;
    showAge?: boolean;
    color?: string | null;
  } = $props();

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

  {#if showStatus}
    <span class="status">{issue.status}</span>
  {/if}

  <!-- Hien TEN GOI chu khong phai chu viet tat: "TN" bat nguoi doc phai giai ma,
       "Tuan" thi nhan ra ngay. Tooltip giu ten day du cua Jira. -->
  <span class="who" title={issue.assigneeDisplay ?? "chua giao"}>
    {#if color}<span class="who-dot" style:background={color}></span>{/if}
    <span class="who-name">{issue.shortName}</span>
  </span>

  {#if showAge}
    <span class="metric num" class:warn={issue.isOld} title="song tu luc tao">
      {#if issue.isOld}<span class="dot dot-warning"></span>{/if}{issue.ageDays}d
    </span>
  {/if}

  <span
    class="metric num"
    class:crit={issue.isStale}
    title="tu lan cap nhat gan nhat"
  >
    {#if issue.isStale}<span class="dot dot-critical"></span>{/if}{issue.idleDays}d
  </span>
</button>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 3px 4px;
    border-radius: 5px;
    font-size: 11px;
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--raised);
  }

  .key {
    font-weight: 600;
    color: var(--text-primary);
    flex: none;
    width: 74px;
  }

  .status {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
  }

  .who {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    width: 62px;
    color: var(--text-secondary);
    font-weight: 500;
  }
  .who-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who-dot {
    width: 6px;
    height: 6px;
    border-radius: 2px;
    flex: none;
  }

  /* Chu luon deo token chu; cham mau chi la dau hieu phu di kem con so. */
  .metric {
    flex: none;
    width: 38px;
    text-align: right;
    color: var(--text-muted);
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 3px;
  }
  .metric.warn,
  .metric.crit {
    color: var(--text-primary);
    font-weight: 600;
  }
  .dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    flex: none;
  }
  .dot-critical {
    background: var(--status-critical);
  }
  .dot-warning {
    background: var(--status-warning);
  }
</style>
