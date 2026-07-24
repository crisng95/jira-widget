<script lang="ts">
  // Dung chung cho ca ba hang doi: cho test, cho duyet, cho release.
  //
  // Status quyet dinh "dang cho gi", field nguoi quyet dinh "cho AI".
  // Test loc theo QCs, Duyet loc theo Approvers — hai vai tro KHAC nhau
  // (tren project: Approvers = alex.lee, QCs = blake.kim). Release khong
  // loc theo nguoi vi do la viec cua ca team.
  import type { Queue } from "../types";
  import TicketRow from "./TicketRow.svelte";

  let {
    queue,
    titleMine,
    titleAll,
    emptyMine,
    emptyAll,
    colors,
  }: {
    queue: Queue;
    titleMine: string;
    titleAll: string;
    emptyMine: string;
    emptyAll: string;
    colors: Map<string, string>;
  } = $props();

  let mine = $derived(queue.scope === "mine");
</script>

{#if queue.visible}
  <div class="section">
    <div class="section-head">
      <span class="section-title">{mine ? titleMine : titleAll}</span>
      <span class="count-pill">{queue.items.length}</span>
    </div>

    {#if queue.items.length === 0}
      <p class="empty">{mine ? emptyMine : emptyAll}</p>
    {:else}
      <div>
        {#each queue.items as issue (issue.key)}
          <TicketRow
            {issue}
            showStatus={true}
            color={issue.assignee ? colors.get(issue.assignee) : null}
          />
        {/each}
      </div>
    {/if}
  </div>
{/if}
