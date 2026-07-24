<script lang="ts">
  import type { Risks } from "../types";
  import TicketRow from "./TicketRow.svelte";

  let {
    risks,
    staleDaysLabel,
    colors,
  }: {
    risks: Risks;
    staleDaysLabel: string;
    colors: Map<string, string>;
  } = $props();

  // Nhom "sap het sprint" chi la canh bao khi sprint that su sap het;
  // ngoai luc do no chi la con so tham khao nen khong mo san.
  let open = $state<Record<string, boolean>>({
    stale: true,
    ending: false,
    unassigned: false,
  });

  let groups = $derived(
    [
      {
        id: "stale",
        label: `Dung im > ${staleDaysLabel}`,
        tone: "critical",
        items: risks.stale,
      },
      {
        id: "ending",
        label: risks.sprintEndingSoon
          ? "Chua xong, sap het sprint"
          : "Chua xong",
        tone: risks.sprintEndingSoon ? "warning" : "neutral",
        items: risks.endingSoon,
      },
      {
        id: "unassigned",
        label: "Chua co assignee",
        tone: "neutral",
        items: risks.unassigned,
      },
    ].filter((g) => g.items.length > 0),
  );
</script>

<div class="section">
  <div class="section-head">
    <span class="section-title">Canh bao</span>
    {#if risks.count > 0}
      <span class="count-pill">{risks.count}</span>
    {/if}
  </div>

  {#if groups.length === 0}
    <p class="ok"><span class="dot dot-good"></span> Khong co canh bao nao</p>
  {:else}
    {#each groups as g (g.id)}
      <div class="group">
        <button class="ghead" onclick={() => (open[g.id] = !open[g.id])}>
          <span
            class="dot"
            class:dot-critical={g.tone === "critical"}
            class:dot-warning={g.tone === "warning"}
            class:dot-neutral={g.tone === "neutral"}
          ></span>
          <span class="glabel">{g.label}</span>
          <span class="gcount num">{g.items.length}</span>
          <span class="caret">{open[g.id] ? "▾" : "▸"}</span>
        </button>

        {#if open[g.id]}
          <div class="glist">
            {#each g.items.slice(0, 8) as issue (issue.key)}
              <TicketRow
                {issue}
                showStatus={true}
                color={issue.assignee ? colors.get(issue.assignee) : null}
              />
            {/each}
            {#if g.items.length > 8}
              <p class="more muted num">... con {g.items.length - 8} ticket nua</p>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .ok {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .group + .group {
    margin-top: 2px;
  }

  .ghead {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 3px 4px;
    border-radius: 5px;
    font-size: 11px;
    cursor: pointer;
  }
  .ghead:hover {
    background: var(--raised);
  }

  .glabel {
    flex: 1;
    text-align: left;
    color: var(--text-secondary);
  }
  .gcount {
    font-weight: 600;
    color: var(--text-primary);
  }
  .caret {
    color: var(--text-muted);
    font-size: 9px;
    width: 10px;
  }

  .glist {
    padding-left: 12px;
  }

  .more {
    font-size: 10px;
    margin: 2px 0 0 4px;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
  }
  .dot-critical {
    background: var(--status-critical);
  }
  .dot-warning {
    background: var(--status-warning);
  }
  .dot-neutral {
    background: var(--baseline);
  }
  .dot-good {
    background: var(--status-good);
  }
</style>
