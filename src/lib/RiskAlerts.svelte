<script lang="ts">
  import type { Risks } from "../types";
  import TicketRow from "./TicketRow.svelte";
  import { t } from "../i18n.svelte";

  let {
    risks,
    staleDays,
    colors,
  }: {
    risks: Risks;
    staleDays: number;
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
        label: t("staleGroup", { d: staleDays }),
        tone: "critical",
        items: risks.stale,
      },
      {
        id: "ending",
        label: risks.sprintEndingSoon ? t("endingSoonGroup") : t("notDoneGroup"),
        tone: risks.sprintEndingSoon ? "warning" : "neutral",
        items: risks.endingSoon,
      },
      {
        id: "unassigned",
        label: t("unassignedGroup"),
        tone: "neutral",
        items: risks.unassigned,
      },
    ].filter((g) => g.items.length > 0),
  );
</script>

<div class="section">
  <div class="section-head">
    <span class="section-title">{t("alerts")}</span>
    {#if risks.count > 0}
      <span class="count-pill hot num">{risks.count}</span>
    {/if}
  </div>

  {#if groups.length === 0}
    <p class="ok"><span class="dot dot-good"></span> {t("noAlerts")}</p>
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
                color={issue.assignee ? colors.get(issue.assignee) : null}
              />
            {/each}
            {#if g.items.length > 8}
              <p class="more muted num">{t("moreTickets", { n: g.items.length - 8 })}</p>
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
    gap: 7px;
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
    padding: 3px 0;
  }

  .group + .group {
    margin-top: var(--sp-xs);
  }

  .ghead {
    display: flex;
    align-items: center;
    gap: 7px;
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
    font-weight: 640;
    color: var(--text-primary);
  }
  .caret {
    color: var(--text-muted);
    font-size: 9px;
    width: 10px;
  }

  /* Ticket con thut vao duoi nhom cua no */
  .glist {
    padding-left: var(--pad-x);
  }

  .more {
    font-size: 10px;
    margin: 2px 0 0 4px;
  }
</style>
