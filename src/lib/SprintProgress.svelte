<script lang="ts">
  import type { Progress, PointTotals } from "../types";
  import { fmtNum } from "../types";
  import { t } from "../i18n.svelte";

  let {
    progress,
    points,
    sprintContext,
    onlyMe = false,
  }: {
    progress: Progress;
    points: PointTotals;
    sprintContext: Progress;
    onlyMe?: boolean;
  } = $props();

  // O Only Me, "37/46 · 80%" van phai co mat — khong thi member khong biet
  // minh dang dung o dau so voi team. O Team mode hai con so nay trung nhau
  // nen hien lai lan nua chi la nhieu.
  let boiCanh = $derived(onlyMe ? sprintContext : null);

  // Thang bac co thu tu -> dung ordinal ramp 1 hue, khong dung mau categorical.
  // "Cho release" tach rieng khoi "Xong" vi Jira xep no vao category Done
  // nhung thuc te ticket moi chi dang cho duyet release.
  let stages = $derived([
    { key: "closed", label: t("stDone"), n: progress.closed, color: "var(--stage-closed)" },
    { key: "release", label: t("stRelease"), n: progress.pendingRelease, color: "var(--stage-release)" },
    { key: "wip", label: t("stWip"), n: progress.inProgress, color: "var(--stage-wip)" },
    { key: "todo", label: t("stTodo"), n: progress.todo, color: "var(--stage-todo)" },
  ].filter((s) => s.n > 0));
</script>

<div class="section">
  <div class="section-head">
    <span class="section-title">{onlyMe ? t("mine") : t("progress")}</span>
    <span class="headline">
      {progress.done}/{progress.total} · {progress.percent}%
    </span>
  </div>

  {#if boiCanh}
    <p class="context num">
      {t("ctxSprint", { d: boiCanh.done, t: boiCanh.total, p: boiCanh.percent })}
    </p>
  {/if}

  <!-- Khoang ho 2px bang mau nen de tach cac doan, khong ve vien -->
  <div
    class="track"
    role="img"
    aria-label={t("progressAria", { d: progress.done, t: progress.total })}
  >
    {#each stages as s (s.key)}
      <span
        class="seg"
        style:flex-grow={s.n}
        style:background={s.color}
        title="{s.label}: {s.n}"
      ></span>
    {/each}
  </div>

  <!-- Legend luon co khi tu 2 doan tro len -->
  <ul class="legend">
    {#each stages as s (s.key)}
      <li>
        <span class="sw" style:background={s.color}></span>
        <span class="lb">{s.label}</span>
        <span class="n num">{s.n}</span>
      </li>
    {/each}
  </ul>

  <!-- Hai pham vi, ghi ro bang CHU. Truoc day chi hien mot con so tinh tren
       ticket dang mo, doc nham thanh diem ca sprint. Mau so van bat buoc vi
       du lieu that rat thua. -->
  <table class="points">
    <thead>
      <tr>
        <th></th>
        <th class="num">Σ SP</th>
        <th class="num">Σ Score</th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <!-- O Only Me hang nay la tong CUA MINH, khong phai ca sprint: `points`
             duoc tinh duoi moc loc. De nguyen chu "cả sprint" thi no da voi
             dong bối cảnh ngay phia tren — hai con so khac nhau cung mot nhan. -->
        <td class="scope">{onlyMe ? t("scopeMine") : t("scopeSprint")}</td>
        <td class="num">
          <b>{fmtNum(points.sprint.spSum)}</b>
          <span class="den">{points.sprint.spFilled}/{points.sprint.denominator}</span>
        </td>
        <td class="num">
          <b>{fmtNum(points.sprint.scoreSum)}</b>
          <span class="den">{points.sprint.scoreFilled}/{points.sprint.denominator}</span>
        </td>
      </tr>
      <tr>
        <td class="scope">{t("scopeOpen")}</td>
        <td class="num">
          {fmtNum(points.open.spSum)}
          <span class="den">{points.open.spFilled}/{points.open.denominator}</span>
        </td>
        <td class="num">
          {fmtNum(points.open.scoreSum)}
          <span class="den">{points.open.scoreFilled}/{points.open.denominator}</span>
        </td>
      </tr>
    </tbody>
  </table>
</div>

<style>
  /* Boi canh la thong tin phu -> muted, nam duoi con so chinh chu khong
     canh tranh voi no. */
  .context {
    margin: -3px 0 7px;
    font-size: 9.5px;
    color: var(--text-muted);
  }

  .track {
    display: flex;
    gap: var(--sp-xs);
    height: 8px;
    margin-bottom: var(--sp-lg);
  }
  .seg {
    display: block;
    min-width: 3px;
  }
  /* dau mut bo tron o hai dau thanh, cac doan giua vuong */
  .seg:first-child {
    border-radius: var(--r-sm) 0 0 var(--r-sm);
  }
  .seg:last-child {
    border-radius: 0 var(--r-sm) var(--r-sm) 0;
  }
  .seg:only-child {
    border-radius: var(--r-sm);
  }

  .legend {
    list-style: none;
    margin: 0 0 9px;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-sm) var(--sp-xl);
    font-size: 10px;
  }
  .legend li {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .sw {
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }
  .lb {
    color: var(--text-secondary);
  }
  .n {
    font-weight: 640;
    color: var(--text-primary);
  }

  /* Bang nho dong vai tro "table view": moi con so deu doc duoc bang chu,
     khong phai suy tu bieu do. */
  .points {
    width: 100%;
    border-collapse: collapse;
    font-size: 10px;
    color: var(--text-secondary);
  }
  .points th {
    font-weight: 500;
    color: var(--text-muted);
    text-align: right;
    padding-bottom: 3px;
    font-size: 9.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .points th:first-child {
    text-align: left;
  }
  .points td {
    padding: 2px 0;
    text-align: right;
  }
  .points td.scope {
    text-align: left;
    color: var(--text-muted);
  }
  .points b {
    color: var(--text-primary);
  }
  .points .den {
    margin-left: 5px;
    font-size: 9px;
    color: var(--text-muted);
  }
</style>
