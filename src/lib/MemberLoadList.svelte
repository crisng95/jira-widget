<script lang="ts">
  import type { MemberLoad } from "../types";
  import { OTHER_COLOR, memberLabel } from "../types";
  import { t } from "../i18n.svelte";

  let {
    members,
    colors,
  }: { members: MemberLoad[]; colors: Map<string, string> } = $props();

  // Mau dinh danh nam o chip initials; thanh ben duoi la THANH TIEN DO
  // done/total, khong phai thanh khoi luong. Phan done to xanh --status-good
  // (status thuc su: xong = tot), phan con lai de tro track.
  let totalAll = $derived(members.reduce((s, m) => s + m.total, 0));
  let doneAll = $derived(members.reduce((s, m) => s + m.done, 0));

  function subLine(m: MemberLoad): string {
    const parts = m.byStatus
      .filter((s) => s.status !== "Closed")
      .map((s) => `${s.count} ${s.status}`);
    return [t("left", { n: m.open }), ...parts].join(" · ");
  }
</script>

<div class="section">
  <div class="section-head">
    <span class="section-title">{t("byMember")}</span>
    <span class="count-pill num">{doneAll}/{totalAll}</span>
  </div>

  {#if members.length === 0}
    <p class="empty">{t("noTicketsSprint")}</p>
  {:else}
    <ul>
      {#each members as m (m.name)}
        <li class="mrow">
          <span
            class="av"
            class:none={m.isUnassigned}
            style:background={m.isUnassigned
              ? "var(--raised)"
              : (colors.get(m.name) ?? OTHER_COLOR)}
            title={m.isUnassigned ? t("unassigned") : m.display}>{m.initials}</span
          >
          <span class="body">
            <span class="l1">
              <span class="nm" class:ghost={m.isUnassigned} title={m.isUnassigned ? t("unassigned") : m.display}
                >{m.isUnassigned ? t("unassigned") : memberLabel(m)}</span
              >{#if m.isMe}<span class="metag">{t("you")}</span>{/if}
              <!-- So luon hien ben canh: thanh khong bao gio la cach duy nhat doc -->
              <span class="fig num">
                <span class="c"><b>{m.done}</b>/{m.total}</span>
                <span class="p">{m.donePercent}%</span>
                {#if m.total > 0 && m.done === m.total}<span
                    class="tick"
                    title={t("allDoneTick")}>✓</span
                  >{/if}
              </span>
            </span>
            <span class="track" title="{m.done}/{m.total}">
              <span
                class="fill"
                class:ghostfill={m.isUnassigned}
                style:width="{m.donePercent}%"
              ></span>
            </span>
            {#if m.open > 0}
              <span class="l2 num">{subLine(m)}</span>
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
  }

  /* Vach 0.5px giua cac hang — nguoi la don vi doc, khong phai dong chu */
  .mrow {
    display: grid;
    grid-template-columns: 22px 1fr;
    gap: 9px;
    padding: 7px 0;
    border-top: 0.5px solid var(--gridline);
  }
  .mrow:first-child {
    border-top: none;
    padding-top: 0;
  }
  .mrow:last-child {
    padding-bottom: 0;
  }

  .av {
    width: 22px;
    height: 22px;
    border-radius: var(--r-md);
    display: grid;
    place-items: center;
    font-size: 9px;
    font-weight: 700;
    color: #fff;
    letter-spacing: 0.02em;
  }
  .av.none {
    color: var(--text-muted);
  }

  .body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .l1 {
    display: flex;
    align-items: center;
    gap: var(--sp-md);
    font-size: 11.5px;
  }
  .nm {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    color: var(--text-primary);
  }
  .nm.ghost {
    color: var(--text-muted);
    font-weight: 400;
  }

  .metag {
    font-size: 8.5px;
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: 1px 5px;
    border-radius: var(--r-sm);
    background: var(--you-bg);
    color: var(--you-fg);
    flex: none;
  }

  .fig {
    margin-left: auto;
    display: inline-flex;
    align-items: baseline;
    gap: var(--sp-md);
    flex: none;
  }
  .fig .c {
    font-size: 11px;
    font-weight: 660;
    color: var(--text-secondary);
  }
  .fig .c b {
    color: var(--text-primary);
  }
  .fig .p {
    font-size: 10px;
    color: var(--text-muted);
  }
  .tick {
    color: var(--status-good);
    font-size: 11px;
    font-weight: 700;
  }

  .track {
    display: block;
    height: 4px;
    background: var(--raised);
    border-radius: 3px;
    overflow: hidden;
  }
  .fill {
    display: block;
    height: 100%;
    background: var(--status-good);
    border-radius: 3px;
  }
  .fill.ghostfill {
    background: var(--baseline);
  }

  .l2 {
    font-size: 9.5px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
