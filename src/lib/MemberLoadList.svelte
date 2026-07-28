<script lang="ts">
  import type { MemberLoad } from "../types";
  import { OTHER_COLOR, memberLabel } from "../types";
  import { t } from "../i18n.svelte";

  let {
    members,
    colors,
  }: { members: MemberLoad[]; colors: Map<string, string> } = $props();

  // Mau dinh danh nam o chip initials; thanh ben duoi la THANH TIEN DO cua
  // rieng nguoi do, xep lop theo status category: xong = xanh --status-good,
  // dang lam = vang --status-warning, chua lam = xam track. Ba category cua
  // Jira (done / indeterminate / new) roi nhau va phu kin, nen
  // done + inProgress + todo == total — khong can doan phan du.
  //
  // Dung flex-grow thay vi width:% de khong bao gio tran: ba con so lam tron
  // rieng le co the cong lai thanh 101%.
  let totalAll = $derived(members.reduce((s, m) => s + m.total, 0));
  let doneAll = $derived(members.reduce((s, m) => s + m.done, 0));
  let wipAll = $derived(members.reduce((s, m) => s + m.inProgress, 0));
  let todoAll = $derived(members.reduce((s, m) => s + m.todo, 0));

  let legend = $derived(
    [
      { key: "done", label: t("stDone"), n: doneAll, color: "var(--status-good)" },
      { key: "wip", label: t("stWip"), n: wipAll, color: "var(--status-warning)" },
      { key: "todo", label: t("stTodo"), n: todoAll, color: "var(--baseline)" },
    ].filter((s) => s.n > 0),
  );

  function segs(m: MemberLoad) {
    return [
      { key: "done", n: m.done, color: "var(--status-good)", label: t("stDone") },
      { key: "wip", n: m.inProgress, color: "var(--status-warning)", label: t("stWip") },
      { key: "todo", n: m.todo, color: "var(--baseline)", label: t("stTodo") },
    ].filter((s) => s.n > 0);
  }

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
    <!-- Chu giai kem tong cua ca danh sach: doc mot dong la biet ba mau nghia
         gi VA team dang dung o dau, khong phai cong nham tu cac thanh. -->
    <ul class="legend">
      {#each legend as s (s.key)}
        <li>
          <span class="sw" style:background={s.color}></span>
          <span class="lb">{s.label}</span>
          <span class="n num">{s.n}</span>
        </li>
      {/each}
    </ul>

    <!-- Vang bien mat co hai nghia rat khac nhau: khong ai lam, hay da xong
         het. Noi thang ra truong hop dau, dung de nguoi doc tu suy. -->
    {#if wipAll === 0 && todoAll > 0}
      <p class="nowip">{t("nobodyWip")}</p>
    {/if}

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
              >{#if m.isMe}<span class="metag">{t("you")}</span>{/if}{#if m.inProgress > 0}<span
                  class="wiptag num"
                  title={t("wipTag", { n: m.inProgress })}
                  ><i class="wipdot"></i>{m.inProgress}</span
                >{/if}
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
            <span
              class="track"
              class:ghost={m.isUnassigned}
              role="img"
              aria-label={t("barBreak", { d: m.done, w: m.inProgress, o: m.todo })}
              title={t("barBreak", { d: m.done, w: m.inProgress, o: m.todo })}
            >
              {#each segs(m) as s (s.key)}
                <span
                  class="seg"
                  style:flex-grow={s.n}
                  style:background={s.color}
                  title="{s.label}: {s.n}"
                ></span>
              {/each}
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

  /* Chu giai chung, dat ngay duoi tieu de section */
  .legend {
    flex-direction: row;
    flex-wrap: wrap;
    gap: 3px var(--sp-lg);
    margin: -2px 0 8px;
    font-size: 9.5px;
  }
  .legend li {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .sw {
    width: 7px;
    height: 7px;
    border-radius: 2px;
    flex: none;
  }
  .legend .lb {
    color: var(--text-muted);
  }
  .legend .n {
    font-weight: 640;
    color: var(--text-secondary);
  }

  .nowip {
    margin: -4px 0 8px;
    font-size: 9.5px;
    color: var(--text-muted);
  }

  /* Chip "dang lam": cham vang + so, doc duoc ngay canh ten nen khong phai
     do do dai doan vang tren thanh moi biet ai dang chay viec. */
  .wiptag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    flex: none;
    font-size: 9px;
    font-weight: 700;
    padding: 1px 5px 1px 4px;
    border-radius: var(--r-sm);
    background: var(--wip-bg);
    color: var(--wip-fg);
  }
  .wipdot {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--status-warning);
    flex: none;
  }

  /* Thanh xep lop: xong | dang lam | chua lam. Khe 1.5px cung mau nen tach
     cac doan ma khong can ve vien. */
  .track {
    display: flex;
    gap: 1.5px;
    height: 5px;
    background: var(--raised);
    border-radius: 3px;
    overflow: hidden;
  }
  .seg {
    display: block;
    min-width: 3px;
  }
  .track.ghost .seg {
    opacity: 0.5;
  }

  .l2 {
    font-size: 9.5px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
