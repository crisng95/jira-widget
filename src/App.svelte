<script lang="ts">
  // Luu y: KHONG duoc dat bien ten `state` — Svelte 5 se hieu `$state` la
  // store subscription chu khong phai rune. Dung `panel`.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { PanelState } from "./types";
  import { colorMap } from "./types";

  import Header from "./lib/Header.svelte";
  import RiskAlerts from "./lib/RiskAlerts.svelte";
  import SprintProgress from "./lib/SprintProgress.svelte";
  import Allocation from "./lib/Allocation.svelte";
  import MemberLoadList from "./lib/MemberLoadList.svelte";
  import QueueSection from "./lib/QueueSection.svelte";
  import TicketRow from "./lib/TicketRow.svelte";

  let panel = $state<PanelState | null>(null);
  let compact = $state(false);
  let showAll = $state(false);

  let snap = $derived(panel?.snapshot ?? null);
  let colors = $derived(
    snap ? colorMap(snap.colorOrder) : new Map<string, string>(),
  );

  let onlyMe = $derived(snap?.displayMode === "only_me");
  // Hai kieu "rong" khac han nhau ve y nghia nen phai noi hai cau khac nhau:
  // khong duoc giao viec != da lam xong het viec.
  let khongCoViec = $derived(onlyMe && snap !== null && snap.issues.length === 0);
  let xongHetViec = $derived(
    onlyMe && snap !== null && snap.issues.length > 0 && snap.openIssues.length === 0,
  );

  async function veTeam() {
    try {
      await invoke("set_display_mode", { mode: "team" });
    } catch (e) {
      console.error("set_display_mode loi", e);
    }
  }

  onMount(() => {
    invoke<PanelState>("get_state")
      .then((s) => (panel = s))
      .catch((e) => console.error("get_state loi", e));

    const un = listen<PanelState>("panel://state", (ev) => {
      panel = ev.payload;
    });

    // Tray bam "Thu gon / mo rong" -> lat o day de trang thai chi co MOT nguon
    const unCompact = listen("panel://toggle-compact", () => {
      compact = !compact;
      invoke("set_compact", { compact }).catch((e) =>
        console.error("set_compact loi", e),
      );
    });

    return () => {
      un.then((f) => f());
      unCompact.then((f) => f());
    };
  });
</script>

<div class="panel">
  {#if panel === null}
    <div class="boot">Dang tai...</div>
  {:else}
    <Header {panel} bind:compact />

    {#if !compact}
      <div class="scroll">
        {#if panel.errorKind === "auth"}
          <!-- Loi auth khong duoc im lang: panel trang thi anh tuong sprint rong -->
          <div class="section notice">
            <p class="ntitle"><span class="dot dot-critical"></span> Token het han</p>
            <p class="nbody">{panel.errorMessage}</p>
            <p class="nbody muted">
              Cap lai PAT trong Jira roi chay:<br />
              <code>jira-widget --set-token</code>
            </p>
          </div>
        {:else if !panel.ok && !snap}
          <div class="section notice">
            <p class="ntitle">
              <span class="dot dot-critical"></span> Khong ket noi duoc Jira
            </p>
            <p class="nbody">{panel.errorMessage}</p>
            <p class="nbody muted num">
              Da thu {panel.consecutiveFailures} lan. Kiem tra VPN / mang.
            </p>
          </div>
        {:else if panel.noActiveSprint}
          <div class="section notice">
            <p class="ntitle">
              <span class="dot dot-neutral"></span> Chua co sprint dang chay
            </p>
            <p class="nbody muted">
              Board dang o giua hai sprint. Panel se tu bat lai khi sprint moi mo.
            </p>
          </div>
        {/if}

        {#if snap}
          {#if !panel.ok}
            <!-- Van hien du lieu cu, chi ghi ro no la du lieu cu -->
            <div class="stalebar">
              <span class="dot dot-warning"></span>
              Du lieu cu — dang thu ket noi lai ({panel.consecutiveFailures} lan hong)
            </div>
          {/if}

          <RiskAlerts
            risks={snap.risks}
            staleDaysLabel="{panel.staleDays} ngay"
            {colors}
          />
          {#if khongCoViec}
            <!-- Man hinh rong PHAI tu giai thich. Bat Only Me roi thay panel
                 trang la luc nguoi dung tuong app hong, trong khi su that chi
                 la sprint nay chua giao viec cho ho. -->
            <div class="section notice">
              <p class="ntitle">
                <span class="dot dot-neutral"></span> Bạn không có ticket nào trong sprint này
              </p>
              <p class="nbody muted num">
                Cả sprint đang {snap.sprintContext.done}/{snap.sprintContext.total} ·
                {snap.sprintContext.percent}%.
              </p>
              <button class="backbtn" onclick={veTeam}>Xem cả team</button>
            </div>
          {:else}
            <SprintProgress
              progress={snap.progress}
              points={snap.points}
              sprintContext={snap.sprintContext}
              {onlyMe}
            />
            <!-- Donut phan bo va bang tai member la de so sanh GIUA nguoi voi
                 nguoi. Con dung mot nguoi thi donut luon 100% mot mau va bang
                 tai chi con mot dong — hai o vo nghia chiem cho. -->
            {#if !onlyMe}
              <Allocation members={snap.byAssignee} {colors} />
              <MemberLoadList members={snap.byAssignee} {colors} />
            {/if}
          {/if}
          <QueueSection
            queue={snap.testQueue}
            titleMine="Cần tôi test"
            titleAll="Đang chờ test"
            emptyMine="Không có ticket nào chờ bạn test."
            emptyAll="Không có ticket nào đang chờ test."
            {colors}
          />
          <QueueSection
            queue={snap.reviewQueue}
            titleMine="Cần tôi duyệt"
            titleAll="Đang chờ duyệt"
            emptyMine="Không có ticket nào chờ bạn duyệt."
            emptyAll="Không có ticket nào đang chờ duyệt."
            {colors}
          />
          <!-- Chờ release không lọc theo người ở BẤT KỲ mode nào. Ở Only Me mọi
               thứ khác trên panel đều đã lọc, nên tiêu đề trần "Chờ release" sẽ
               bị đọc thành "của tôi" — phải tự nói ra là cả team. -->
          <QueueSection
            queue={snap.releaseQueue}
            titleMine="Chờ release"
            titleAll={onlyMe ? "Chờ release · cả team" : "Chờ release"}
            emptyMine="Không có ticket nào chờ release."
            emptyAll={onlyMe
              ? "Cả team không có ticket nào chờ release."
              : "Không có ticket nào chờ release."}
            {colors}
          />

          {#if !khongCoViec}
            <div class="section">
              <div class="section-head">
                <button class="alltoggle" onclick={() => (showAll = !showAll)}>
                  <span class="section-title">
                    {onlyMe ? "Việc của tôi" : "Tất cả"} ({snap.openIssues.length} chưa xong)
                  </span>
                  <span class="caret">{showAll ? "▾" : "▸"}</span>
                </button>
                <span class="count-pill num" title="tuoi trung vi / lon nhat">
                  p50 {snap.ageStats.medianAge}d · max {snap.ageStats.maxAge}d
                </span>
              </div>
              {#if xongHetViec}
                <!-- Khong gap sau caret: xong het viec la tin tot, dang duoc
                     noi thang chu khong bat nguoi ta bam ra moi thay. -->
                <p class="doneall">Bạn đã xong hết việc trong sprint ✓</p>
              {:else if showAll}
                {#each snap.openIssues as issue (issue.key)}
                  <TicketRow
                    {issue}
                    showStatus={true}
                    showAge={true}
                    color={issue.assignee ? colors.get(issue.assignee) : null}
                  />
                {/each}
              {/if}
            </div>
          {/if}
        {/if}
      </div>
    {:else if snap}
      <!-- Compact: chi giu tien do + so canh bao -->
      <div class="cstrip">
        <span class="cnum num">{snap.progress.done}/{snap.progress.total}</span>
        <span class="ctrack">
          <span class="cfill" style:width="{snap.progress.percent}%"></span>
        </span>
        <span class="cwarn num" class:hot={snap.risks.count > 0}>
          {#if snap.risks.count > 0}<span class="dot dot-critical"></span>{/if}
          {snap.risks.count}
        </span>
      </div>
    {/if}
  {/if}
</div>

<style>
  .boot {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--text-muted);
    font-size: 11px;
  }

  .notice .ntitle {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 4px;
    font-size: 11px;
    font-weight: 600;
  }
  .notice .nbody {
    margin: 0 0 3px;
    font-size: 10px;
    color: var(--text-secondary);
    word-break: break-word;
  }
  code {
    font-size: 9.5px;
  }

  .stalebar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    font-size: 10px;
    color: var(--text-secondary);
    background: var(--raised);
  }

  .alltoggle {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    cursor: pointer;
  }

  .doneall {
    margin: 2px 0 0;
    font-size: 10px;
    color: var(--status-good);
  }

  .backbtn {
    margin-top: 6px;
    padding: 3px 9px;
    border-radius: 5px;
    background: var(--raised);
    color: var(--text-primary);
    font-size: 10px;
    font-weight: 600;
    cursor: pointer;
  }
  .backbtn:hover {
    background: var(--border);
  }
  .caret {
    color: var(--text-muted);
    font-size: 9px;
  }

  .cstrip {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px 10px;
  }
  .cnum {
    font-size: 11px;
    font-weight: 600;
  }
  .ctrack {
    flex: 1;
    height: 8px;
    background: var(--raised);
    border-radius: 4px;
    overflow: hidden;
  }
  .cfill {
    display: block;
    height: 100%;
    background: var(--stage-closed);
    border-radius: 0 4px 4px 0;
  }
  .cwarn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .cwarn.hot {
    color: var(--text-primary);
    font-weight: 600;
  }

  .dot {
    width: 6px;
    height: 6px;
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
</style>
