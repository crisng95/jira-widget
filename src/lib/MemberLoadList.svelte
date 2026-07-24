<script lang="ts">
  import type { MemberLoad } from "../types";
  import { OTHER_COLOR } from "../types";

  let {
    members,
    colors,
  }: { members: MemberLoad[]; colors: Map<string, string> } = $props();

  // Mau dinh danh nam o chip initials; thanh ben duoi la THANH TIEN DO
  // done/total, khong phai thanh khoi luong. Phan done to xanh --status-good
  // (status thuc su: xong = tot), phan con lai de tro track.
  let totalAll = $derived(members.reduce((s, m) => s + m.total, 0));
  let doneAll = $derived(members.reduce((s, m) => s + m.done, 0));
</script>

<div class="section">
  <div class="section-head">
    <span class="section-title">Tien do theo member</span>
    <span class="count-pill num">{doneAll}/{totalAll}</span>
  </div>

  {#if members.length === 0}
    <p class="empty">Sprint chua co ticket nao.</p>
  {:else}
    <ul>
      {#each members as m (m.name)}
        <li class:unassigned={m.isUnassigned} class:me={m.isMe}>
          <span
            class="ini"
            style:background={m.isUnassigned
              ? OTHER_COLOR
              : (colors.get(m.name) ?? OTHER_COLOR)}
            title={m.display}>{m.initials}</span
          >
          <span class="body">
            <span class="l1">
              <span class="nm" title={m.display}
                >{m.isUnassigned ? m.display : m.name}{#if m.isMe}<span
                    class="metag">ban</span
                  >{/if}</span
              >
              <!-- So luon hien ben canh: thanh khong bao gio la cach duy nhat doc -->
              <span class="counts num">
                <b>{m.done}</b>/{m.total}
                <span class="pct">{m.donePercent}%</span>
                {#if m.total > 0 && m.done === m.total}<span
                    class="tick"
                    title="xong het">✓</span
                  >{/if}
              </span>
            </span>
            <span class="track" title="{m.done} xong / {m.total} task">
              <span class="fill" style:width="{m.donePercent}%"></span>
            </span>
            {#if m.open > 0}
              <span class="l2 muted">
                con {m.open}: {m.byStatus
                  .filter((s) => s.status !== "Closed")
                  .map((s) => `${s.count} ${s.status}`)
                  .join(" · ")}
              </span>
            {/if}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  li {
    display: flex;
    align-items: flex-start;
    gap: 7px;
  }
  li.unassigned {
    opacity: 0.72;
  }
  /* Dong cua chinh minh: chu dam hon, khong doi mau — mau van la dinh danh */
  li.me .nm {
    color: var(--text-primary);
    font-weight: 600;
  }
  .metag {
    margin-left: 5px;
    font-size: 8.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-muted);
    border: 0.5px solid var(--border);
    border-radius: 3px;
    padding: 0 3px;
  }

  .ini {
    flex: none;
    width: 20px;
    height: 20px;
    border-radius: 5px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 9px;
    font-weight: 700;
    color: #fff;
    letter-spacing: 0.02em;
  }

  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .l1 {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 6px;
    font-size: 11px;
  }
  .nm {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
  }
  .counts {
    flex: none;
    color: var(--text-muted);
  }
  .counts b {
    color: var(--text-primary);
  }
  .pct {
    margin-left: 3px;
    font-size: 10px;
  }
  .tick {
    margin-left: 2px;
    color: var(--status-good);
    font-weight: 700;
  }

  .track {
    display: block;
    height: 6px;
    background: var(--raised);
    border-radius: 3px;
    overflow: hidden;
  }
  .fill {
    display: block;
    height: 100%;
    background: var(--status-good);
    border-radius: 3px;
    min-width: 0;
  }

  .l2 {
    font-size: 9.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
