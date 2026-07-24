<script lang="ts">
  // Phan bo task theo nguoi.
  //
  // Ghi chu thiet ke: donut chi hop de nhin part-to-whole THOANG QUA va toi da
  // 6 lat. Khi cac lat gan bang nhau thi bar ngang doc chinh xac hon han — nen
  // co nut chuyen. Mau bam theo NGUOI (sort alphabet) chu khong theo thu hang,
  // de loc bot member khong lam doi mau nhung nguoi con lai.
  import type { MemberLoad } from "../types";
  import { OTHER_COLOR, memberLabel, shownMembers } from "../types";
  import { t } from "../i18n.svelte";

  let {
    members,
    colors,
  }: { members: MemberLoad[]; colors: Map<string, string> } = $props();

  let mode = $state<"donut" | "bar">("donut");
  let hovered = $state<string | null>(null);

  // So slot mau do `shownMembers`/`colorMap` trong types.ts quyet dinh — de o
  // mot cho duy nhat cho khoi lech nhau.
  const SIZE = 96;
  const STROKE = 13;
  const R = (SIZE - STROKE) / 2;
  const C = 2 * Math.PI * R;
  const GAP = 2; // khoang ho mau nen giua cac lat — khong dung vien de tach

  type Slice = { name: string; label: string; value: number; color: string };

  // Value la TONG task cua ca sprint, khong phai so ticket con ton. Truoc day
  // dung so ticket dang mo nen donut bao "alex.lee 44%" cho nguoi thuc ra
  // chiem 20% khoi luong sprint.
  //
  // `shownMembers` va `colorMap` dung chung mot phep chon nen khong con canh
  // nguoi duoc ve nhung khong duoc cap mau.
  let slices = $derived.by<Slice[]>(() => {
    const shown = shownMembers(members);
    const head: Slice[] = shown.map((m) => ({
      name: m.name,
      // Nhan doc duoc: username (DC) / display name (Cloud) — khoa accountId
      // tho khong bao gio duoc hien ra legend.
      label: memberLabel(m),
      value: m.total,
      color: colors.get(m.name) ?? OTHER_COLOR,
    }));
    const shownNames = new Set(shown.map((m) => m.name));
    const tail = members.filter((m) => !shownNames.has(m.name));
    if (tail.length > 0) {
      head.push({
        name: "__other__",
        label: t("other", { n: tail.length }),
        value: tail.reduce((s, m) => s + m.total, 0),
        color: OTHER_COLOR,
      });
    }
    return head;
  });

  let total = $derived(slices.reduce((s, x) => s + x.value, 0));

  let arcs = $derived.by(() => {
    let acc = 0;
    return slices.map((s) => {
      const len = total > 0 ? (C * s.value) / total : 0;
      const arc = { ...s, len: Math.max(len - GAP, 1), offset: -acc };
      acc += len;
      return arc;
    });
  });

  function pct(v: number): string {
    return total > 0 ? `${Math.round((v / total) * 100)}%` : "0%";
  }
</script>

<div class="section">
  <div class="section-head">
    <span class="section-title">{t("allocation")}</span>
    <div class="toggle">
      <button
        aria-pressed={mode === "donut"}
        onclick={() => (mode = "donut")}>donut</button
      >
      <button aria-pressed={mode === "bar"} onclick={() => (mode = "bar")}
        >bar</button
      >
    </div>
  </div>

  {#if total === 0}
    <p class="empty">{t("noOpenTickets")}</p>
  {:else if mode === "donut"}
    <div class="alloc">
      <svg
        width={SIZE}
        height={SIZE}
        viewBox="0 0 {SIZE} {SIZE}"
        role="img"
        aria-label={t("allocAria", { n: total })}
      >
        <g transform="rotate(-90 {SIZE / 2} {SIZE / 2})">
          {#each arcs as a (a.name)}
            <circle
              role="graphics-symbol"
              aria-label="{a.label}: {a.value}"
              cx={SIZE / 2}
              cy={SIZE / 2}
              r={R}
              fill="none"
              stroke={a.color}
              stroke-width={STROKE}
              stroke-dasharray="{a.len} {C - a.len}"
              stroke-dashoffset={a.offset}
              opacity={hovered && hovered !== a.name ? 0.35 : 1}
              onmouseenter={() => (hovered = a.name)}
              onmouseleave={() => (hovered = null)}
            />
          {/each}
        </g>
        <!-- hero number: dung so ti le, khong dung tabular-nums -->
        <text
          x={SIZE / 2}
          y={SIZE / 2 - 2}
          text-anchor="middle"
          class="hero">{total}</text
        >
        <text x={SIZE / 2} y={SIZE / 2 + 11} text-anchor="middle" class="hero-sub"
          >{t("ticket")}</text
        >
      </svg>

      <!-- Legend luon co, kem direct label + so luong: day cung la phan "relief"
           bat buoc cho 3 hue duoi 3:1 contrast o light mode. -->
      <ul class="legend">
        {#each slices as s (s.name)}
          <li
            class:dim={hovered !== null && hovered !== s.name}
            onmouseenter={() => (hovered = s.name)}
            onmouseleave={() => (hovered = null)}
          >
            <span class="swatch" style:background={s.color}></span>
            <span class="lg-name">{s.label}</span>
            <span class="lg-val num">{s.value}</span>
            <span class="lg-pct num">{pct(s.value)}</span>
          </li>
        {/each}
      </ul>
    </div>
  {:else}
    <ul class="bars">
      {#each slices as s (s.name)}
        <li
          onmouseenter={() => (hovered = s.name)}
          onmouseleave={() => (hovered = null)}
        >
          <span class="bar-name">{s.label}</span>
          <span class="track">
            <span
              class="fill"
              style:width="{total > 0 ? (s.value / total) * 100 : 0}%"
              style:background={s.color}
              style:opacity={hovered && hovered !== s.name ? 0.35 : 1}
            ></span>
          </span>
          <span class="bar-val num">{s.value}</span>
          <span class="bar-pct num">{pct(s.value)}</span>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .alloc {
    display: grid;
    grid-template-columns: 96px 1fr;
    gap: 14px;
    align-items: center;
  }

  svg {
    flex: none;
  }

  .hero {
    fill: var(--text-primary);
    font-size: 20px;
    font-weight: 680;
    /* so tri hero dung chu so ti le, khong dung tabular-nums */
  }
  .hero-sub {
    fill: var(--text-muted);
    font-size: 9px;
  }

  .legend {
    list-style: none;
    margin: 0;
    padding: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .legend li {
    display: grid;
    grid-template-columns: 9px 1fr auto auto;
    align-items: center;
    gap: 7px;
    font-size: 10.5px;
    /* vung hover rong hon dau mau nhieu lan */
    padding: 1px 2px;
    border-radius: var(--r-sm);
  }
  .legend li.dim {
    opacity: 0.45;
  }
  .legend li:hover {
    background: var(--raised);
  }

  .swatch {
    width: 9px;
    height: 9px;
    border-radius: 2px;
  }
  .lg-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
  }
  .lg-val {
    font-weight: 640;
    color: var(--text-primary);
  }
  .lg-pct {
    width: 30px;
    text-align: right;
    font-size: 10px;
    color: var(--text-muted);
  }

  .bars {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-sm);
  }
  .bars li {
    display: flex;
    align-items: center;
    gap: var(--sp-md);
    font-size: 10.5px;
  }
  .bar-name {
    width: 84px;
    flex: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
  }
  .track {
    flex: 1;
    height: 8px;
    background: var(--raised);
    border-radius: var(--r-sm);
    overflow: hidden;
  }
  .fill {
    display: block;
    height: 100%;
    /* dau mut bo tron, neo vao vach goc */
    border-radius: 0 var(--r-sm) var(--r-sm) 0;
    min-width: 3px;
  }
  .bar-val {
    width: 14px;
    text-align: right;
    font-weight: 640;
  }
  .bar-pct {
    width: 30px;
    text-align: right;
    font-size: 10px;
    color: var(--text-muted);
  }
</style>
