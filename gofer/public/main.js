// The front page: an overview of one namespace built from each pipeline's recent runs.
//
// There's no aggregate stats endpoint, so the stats are computed in the browser from every run each pipeline started
// in the activity window. WINDOW_LIMIT caps how many runs one pipeline can return, and the page says so if a pipeline
// ever hits it.

const WINDOW_LIMIT = 500;
const HISTORY_LENGTH = 20;
const ACTIVITY_DAYS = 14;
const DAY_MS = 24 * 60 * 60 * 1000;

const namespace = new URLSearchParams(window.location.search).get("namespace") || "default";

// "all" or "mine"; only offered when signed in.
let runFilter = "all";

// The last data we rendered, so flipping the run filter doesn't have to wait on the network.
let lastEntries = [];

// ---------------------------------------------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------------------------------------------

function pipelinePath(pipelineId) {
  return `/api/namespaces/${encodeURIComponent(namespace)}/pipelines/${encodeURIComponent(pipelineId)}`;
}

async function loadDashboard() {
  const resp = await apiFetch(`/api/namespaces/${encodeURIComponent(namespace)}/pipelines`);
  if (!resp.ok) {
    const message =
      resp.status === 401 || resp.status === 403
        ? `You need a token with access to the "${namespace}" namespace to see it. Use Sign in at the top right.`
        : resp.status === 404
          ? `The "${namespace}" namespace doesn't exist.`
          : `Could not load pipelines (status ${resp.status || "network error"}). Retrying.`;
    const el = document.getElementById("dashboard-error");
    el.textContent = message;
    el.classList.remove("hidden");
    return;
  }
  document.getElementById("dashboard-error").classList.add("hidden");

  const pipelines = resp.data.pipelines;
  const since = startOfDay(Date.now()) - (ACTIVITY_DAYS - 1) * DAY_MS;
  const runs = await Promise.all(pipelines.map((pipeline) => loadPipelineRuns(pipeline.pipeline_id, since)));

  const entries = pipelines.map((pipeline, i) => ({ pipeline, ...runs[i] }));

  lastEntries = entries;
  render(entries);
}

// Everything a pipeline started in the activity window, newest first. Quiet pipelines may not have enough runs in the
// window to fill their history strip, so for those we also grab their latest runs regardless of age.
async function loadPipelineRuns(pipelineId, since) {
  const windowResp = await apiFetch(`${pipelinePath(pipelineId)}/runs?since=${since}&limit=${WINDOW_LIMIT}&reverse=true`);
  const inWindow = windowResp.ok ? windowResp.data.runs : [];
  const truncated = inWindow.length === WINDOW_LIMIT;

  if (inWindow.length >= HISTORY_LENGTH) {
    return { runs: inWindow, truncated };
  }

  const latestResp = await apiFetch(`${pipelinePath(pipelineId)}/runs?limit=${HISTORY_LENGTH}&reverse=true`);
  const latest = latestResp.ok ? latestResp.data.runs : [];
  const seen = new Set(inWindow.map((run) => run.run_id));
  const merged = inWindow.concat(latest.filter((run) => !seen.has(run.run_id)));
  merged.sort((a, b) => b.run_id - a.run_id);
  return { runs: merged, truncated };
}

// The namespace list needs a token, so anonymous visitors just see which namespace they're looking at.
async function loadNamespaces() {
  const resp = await apiFetch("/api/namespaces");
  const namespaces = resp.ok ? resp.data.namespaces : [];

  if (namespaces.length <= 1) {
    setHtml("namespace-picker", `<span class="text-gray-500 dark:text-gray-400">Namespace</span> <span class="font-mono">${escapeHtml(namespace)}</span>`);
    return;
  }

  const options = namespaces
    .map((ns) => `<option value="${escapeHtml(ns.id)}" ${ns.id === namespace ? "selected" : ""}>${escapeHtml(ns.name || ns.id)}</option>`)
    .join("");
  setHtml(
    "namespace-picker",
    `<label class="flex items-center gap-2"><span class="text-gray-500 dark:text-gray-400">Namespace</span>
      <select id="namespace-select" class="rounded border-0 py-1 pl-2 pr-8 text-sm text-gray-900 ring-1 ring-inset ring-gray-300 dark:bg-neutral-900 dark:text-gray-100 dark:ring-neutral-600">${options}</select>
    </label>`
  );
}

// ---------------------------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------------------------

// Collapses a run's state and status into the four outcomes the page charts.
function runOutcome(run) {
  if (String(run.state).toLowerCase() !== "complete") return "running";
  switch (String(run.status).toLowerCase()) {
    case "successful":
      return "successful";
    case "cancelled":
      return "cancelled";
    default:
      return "failed";
  }
}

// Tailwind needs to see full class names in the source, so these are spelled out.
const OUTCOMES = {
  successful: { label: "Successful", bar: "bg-emerald-500" },
  failed: { label: "Failed", bar: "bg-red-500" },
  cancelled: { label: "Cancelled", bar: "bg-slate-400" },
  running: { label: "In progress", bar: "bg-yellow-400" },
};

function runDurationMs(run) {
  if (!run.started) return 0;
  return (run.ended || Date.now()) - run.started;
}

// "48s", "1m 52s", "1h 05m"; short enough for tiles and table cells.
function compactDuration(ms) {
  const secs = Math.floor(Math.max(ms, 0) / 1000);
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${String(secs % 60).padStart(2, "0")}s`;
  return `${Math.floor(secs / 3600)}h ${String(Math.floor((secs % 3600) / 60)).padStart(2, "0")}m`;
}

function median(values) {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

function runDetailsLink(run) {
  const params = new URLSearchParams({ namespace: run.namespace_id, pipeline: run.pipeline_id, run: run.run_id });
  return `run.html?${params}`;
}

function namespaceFlag() {
  return namespace === "default" ? "" : ` --namespace ${namespace}`;
}

function startOfDay(timestamp) {
  const date = new Date(timestamp);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}

// ---------------------------------------------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------------------------------------------

function render(entries) {
  const allRuns = entries.flatMap((entry) => entry.runs);
  const now = Date.now();
  const activitySince = startOfDay(now) - (ACTIVITY_DAYS - 1) * DAY_MS;

  const truncated = entries.filter((entry) => entry.truncated).map((entry) => entry.pipeline.pipeline_id);
  setHtml(
    "sample-note",
    truncated.length
      ? `Some pipelines started more than ${WINDOW_LIMIT} runs in the last ${ACTIVITY_DAYS} days, so their counts are low: ${truncated.map(escapeHtml).join(", ")}.`
      : `Across every pipeline in this namespace over the last ${ACTIVITY_DAYS} days.`
  );

  setHtml("stats", renderStats(allRuns, now));
  setHtml("activity-legend", renderActivityLegend());
  setHtml("activity", renderActivity(allRuns, activitySince));

  const attention = renderAttention(entries);
  document.getElementById("attention-section").classList.toggle("hidden", attention === "");
  setHtml("attention", attention);

  const disabled = entries.filter((entry) => String(entry.pipeline.state).toLowerCase() === "disabled").length;
  setHtml(
    "pipelines-note",
    `${entries.length} pipeline${entries.length === 1 ? "" : "s"}${disabled ? `, ${disabled} disabled` : ""}. Each square is a run, oldest on the left.`
  );
  setHtml("pipelines", renderPipelines(entries));

  renderRecentRuns(allRuns);
}

function renderRecentRuns(allRuns) {
  const filter = document.getElementById("run-filter");
  filter.classList.toggle("hidden", !session);
  const mine = session && runFilter === "mine";

  if (session) {
    const option = (value, label) =>
      `<button type="button" data-run-filter="${value}" class="rounded px-3 py-1 ${runFilter === value ? "bg-gray-100 dark:bg-neutral-700 font-medium" : "text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"}">${label}</button>`;
    setHtml("run-filter", option("all", "All runs") + option("mine", "Started by me"));
  }

  const runs = mine ? allRuns.filter((run) => run.initiator.id === session.id) : allRuns;
  const recent = [...runs].sort((a, b) => b.started - a.started).slice(0, 10);
  setHtml(
    "run-list-body",
    recent.length === 0 && mine
      ? `<tr><td colspan="6" class="px-4 py-8 text-center text-gray-500 dark:text-gray-400">You haven't started any of the recent runs in this namespace.</td></tr>`
      : renderRunTable(recent)
  );
}

const HOUR_MS = 60 * 60 * 1000;

// Run counts for each of the last 24 clock hours, oldest first, ending with the current hour. Also returns how many
// runs started in the 24 hours before that, for comparison.
function hourlyCounts(runs, now) {
  const currentHour = Math.floor(now / HOUR_MS) * HOUR_MS;
  const windowStart = currentHour - 23 * HOUR_MS;
  const counts = new Array(24).fill(0);
  let previous = 0;

  for (const run of runs) {
    if (!run.started) continue;
    if (run.started >= windowStart) {
      const index = Math.floor((run.started - windowStart) / HOUR_MS);
      if (index < 24) counts[index]++;
    } else if (run.started >= windowStart - 24 * HOUR_MS) {
      previous++;
    }
  }

  return { counts, windowStart, previous };
}

// A row of tiny hourly bars, oldest on the left and the current hour in the accent color, captioned so it's clear
// what each bar is. Hovering a bar shows its hour and count. Single series, so no legend.
function hourlyBars(counts, windowStart) {
  const max = Math.max(...counts, 1);

  const bars = counts
    .map((value, i) => {
      const current = i === counts.length - 1;
      const hour = new Date(windowStart + i * HOUR_MS).toLocaleTimeString("en-US", { hour: "numeric" });
      const label = current ? `This hour (${hour})` : hour;
      // Empty hours still get a sliver so the row reads as a full day rather than a few floating bars.
      const height = value === 0 ? "2px" : `${Math.max((value / max) * 100, 12)}%`;
      const color = value === 0 ? "bg-gray-200 dark:bg-neutral-700" : current ? "bg-emerald-500" : "bg-gray-300 dark:bg-neutral-500";
      return `<span title="${label}: ${value} run${value === 1 ? "" : "s"}" class="flex h-full w-[3px] items-end">
          <span class="w-full rounded-t-sm ${color}" style="height:${height}"></span>
        </span>`;
    })
    .join("");

  return `<div class="flex shrink-0 items-end gap-1">
      <span class="text-[10px] leading-none text-gray-500 dark:text-gray-400">hourly</span>
      <div class="flex h-5 items-end gap-px">${bars}</div>
    </div>`;
}

// "↑ 2 vs. prior 24h". Kept in neutral ink because more or fewer runs isn't good or bad on its own.
function runCountDelta(current, previous) {
  const diff = current - previous;
  if (diff === 0) return "Same as the prior 24h";
  return `${diff > 0 ? "↑" : "↓"} ${Math.abs(diff).toLocaleString()} vs. prior 24h`;
}

function renderStats(allRuns, now) {
  // Label and number share the first row and the detail shares the second with any chart, so every tile stays two
  // rows tall and the row of tiles doesn't pick up empty space.
  const tile = (label, value, detail, extra = "") =>
    `<div class="bg-white dark:bg-neutral-800 px-4 py-3.5 space-y-1.5">
      <div class="flex items-baseline justify-between gap-3">
        <p class="text-sm text-gray-600 dark:text-gray-300">${label}</p>
        <p class="text-2xl font-semibold tabular-nums">${value}</p>
      </div>
      <div class="flex items-end justify-between gap-3">
        <p class="min-w-0 text-xs text-gray-500 dark:text-gray-400">${detail}</p>
        ${extra}
      </div>
    </div>`;

  const hourly = hourlyCounts(allRuns, now);
  const lastDay = hourly.counts.reduce((a, b) => a + b, 0);

  const lastWeek = allRuns.filter((run) => run.started >= now - 7 * DAY_MS && runOutcome(run) !== "running");
  const judged = lastWeek.filter((run) => runOutcome(run) !== "cancelled");
  const succeeded = judged.filter((run) => runOutcome(run) === "successful").length;
  const rate = judged.length ? Math.round((succeeded / judged.length) * 100) : null;
  const rateIcon = rate === null ? "" : rate >= 90 ? "✓ " : rate >= 70 ? "△ " : "✗ ";

  const running = allRuns.filter((run) => runOutcome(run) === "running");
  const runningPipelines = [...new Set(running.map((run) => run.pipeline_id))];

  const typical = median(lastWeek.map(runDurationMs));

  return [
    tile("Runs, last 24 hours", lastDay.toLocaleString(), runCountDelta(lastDay, hourly.previous), hourlyBars(hourly.counts, hourly.windowStart)),
    tile(
      "Success rate, last 7 days",
      rate === null ? "-" : `${rate}%`,
      rate === null ? "No finished runs yet" : `${rateIcon}${succeeded} of ${judged.length} finished runs, not counting cancelled`
    ),
    tile(
      "Running now",
      running.length.toLocaleString(),
      runningPipelines.length ? `in ${runningPipelines.map((id) => `<span class="font-mono">${escapeHtml(id)}</span>`).join(", ")}` : "Nothing in progress"
    ),
    tile("Typical run time", lastWeek.length ? compactDuration(typical) : "-", "Median of finished runs, last 7 days"),
  ].join("");
}

function renderActivityLegend() {
  return Object.values(OUTCOMES)
    .map((outcome) => `<span class="flex items-center gap-1.5"><span class="inline-block size-2.5 rounded-sm ${outcome.bar}"></span>${outcome.label}</span>`)
    .join("");
}

// Stacked columns per day. Each column caps at 24px wide with a 2px gap between segments, and hovering a day
// shows its breakdown.
function renderActivity(allRuns, since) {
  const days = [];
  for (let i = 0; i < ACTIVITY_DAYS; i++) {
    days.push({ start: since + i * DAY_MS, counts: { successful: 0, failed: 0, cancelled: 0, running: 0 } });
  }

  for (const run of allRuns) {
    if (!run.started || run.started < since) continue;
    const index = Math.round((startOfDay(run.started) - since) / DAY_MS);
    if (days[index]) days[index].counts[runOutcome(run)]++;
  }

  const totals = days.map((day) => Object.values(day.counts).reduce((a, b) => a + b, 0));
  const busiest = Math.max(...totals);
  if (busiest === 0) {
    return `<p class="py-10 text-center text-sm text-gray-500 dark:text-gray-400">No runs in the last ${ACTIVITY_DAYS} days.</p>`;
  }

  // Round the top of the scale up to a clean number so the gridline label reads naturally.
  const step = busiest <= 5 ? 1 : busiest <= 20 ? 5 : busiest <= 50 ? 10 : 25;
  const scaleMax = Math.ceil(busiest / step) * step;
  const chartHeight = 160;

  const columns = days.map((day, i) => {
    const total = totals[i];
    const date = new Date(day.start);
    const isToday = i === days.length - 1;
    const label = isToday ? "Today" : date.toLocaleDateString("en-US", { month: "short", day: "numeric" });

    // Draw bottom-up in a fixed order so a given outcome always sits in the same place.
    const segments = ["successful", "failed", "cancelled", "running"]
      .filter((key) => day.counts[key] > 0)
      .map((key) => `<div class="${OUTCOMES[key].bar} w-full" style="height:${(day.counts[key] / scaleMax) * chartHeight}px"></div>`);

    const breakdown = Object.entries(day.counts)
      .filter(([, count]) => count > 0)
      .map(([key, count]) => `<div class="flex items-center justify-between gap-4"><span class="flex items-center gap-1.5"><span class="inline-block size-2 rounded-sm ${OUTCOMES[key].bar}"></span>${OUTCOMES[key].label}</span><span class="tabular-nums">${count}</span></div>`)
      .join("");

    return `<div class="group relative flex flex-1 flex-col items-center">
        <div class="flex w-full justify-center rounded-sm group-hover:bg-gray-100 dark:group-hover:bg-neutral-700/50" style="height:${chartHeight}px">
          <div class="flex w-full max-w-[24px] flex-col-reverse gap-[2px] self-end overflow-hidden rounded-t">${segments.join("")}</div>
        </div>
        <span class="mt-2 text-[11px] text-gray-500 dark:text-gray-400 ${i % 2 === (days.length - 1) % 2 ? "" : "invisible"}">${label}</span>
        <div class="pointer-events-none absolute bottom-full z-10 mb-1 hidden w-40 ${i >= days.length - 3 ? "right-0" : "left-0"} rounded-md bg-white dark:bg-neutral-900 p-2 text-xs shadow-lg ring-1 ring-gray-200 dark:ring-neutral-700 group-hover:block">
          <p class="mb-1 font-medium">${date.toLocaleDateString("en-US", { weekday: "short", month: "short", day: "numeric" })}</p>
          ${total ? breakdown : `<p class="text-gray-500 dark:text-gray-400">No runs</p>`}
          <div class="mt-1 flex justify-between border-t border-gray-200 dark:border-neutral-700 pt-1"><span>Total</span><span class="tabular-nums">${total}</span></div>
        </div>
      </div>`;
  });

  return `<div class="relative">
      <div class="pointer-events-none absolute inset-x-0 top-0 flex flex-col justify-between text-[11px] text-gray-400" style="height:${chartHeight}px">
        <div class="border-t border-gray-200 dark:border-neutral-700"><span class="relative -top-2 bg-white dark:bg-neutral-800 pr-1">${scaleMax}</span></div>
        <div class="border-t border-gray-200 dark:border-neutral-700"></div>
      </div>
      <div class="relative flex gap-1 pl-6">${columns.join("")}</div>
    </div>`;
}

// Pipelines whose most recent finished run failed, with the CLI command to dig in.
function renderAttention(entries) {
  return entries
    .map((entry) => {
      const lastFinished = entry.runs.find((run) => runOutcome(run) !== "running");
      if (!lastFinished || runOutcome(lastFinished) !== "failed") return "";

      const name = entry.pipeline.name || entry.pipeline.pipeline_id;
      const reason = lastFinished.status_reason ? escapeHtml(lastFinished.status_reason.description) : "";

      return `<div class="rounded-md border border-gray-200 dark:border-neutral-700 border-l-4 border-l-red-500 p-4 space-y-3">
          <div class="flex items-baseline justify-between gap-4">
            <p class="text-sm"><span class="mr-1">✗</span><span class="font-medium">${escapeHtml(name)}</span>
              <a href="${escapeHtml(runDetailsLink(lastFinished))}" class="text-emerald-600 dark:text-emerald-400 hover:underline">run #${lastFinished.run_id}</a> failed</p>
            <span class="shrink-0 text-xs text-gray-500 dark:text-gray-400" title="${formatTimestamp(lastFinished.ended)}">${relativeTime(lastFinished.ended || lastFinished.started)}</span>
          </div>
          ${reason ? `<p class="text-sm text-gray-600 dark:text-gray-300">${reason}</p>` : ""}
          ${commandBlock(`gofer run debug ${lastFinished.pipeline_id} ${lastFinished.run_id}${namespaceFlag()}`)}
        </div>`;
    })
    .join("");
}

function renderHistory(runs) {
  // Oldest on the left, newest on the right, padded on the left so every card's strip lines up.
  const recent = runs.slice(0, HISTORY_LENGTH).reverse();
  const padding = new Array(HISTORY_LENGTH - recent.length)
    .fill(`<span class="h-6 flex-1 rounded-sm bg-gray-100 dark:bg-neutral-700/50"></span>`)
    .join("");

  const squares = recent
    .map((run) => {
      const outcome = runOutcome(run);
      const title = `#${run.run_id} · ${OUTCOMES[outcome].label.toLowerCase()} · ${compactDuration(runDurationMs(run))} · ${relativeTime(run.started)}`;
      return `<a href="${escapeHtml(runDetailsLink(run))}" title="${escapeHtml(title)}"
          class="h-6 flex-1 rounded-sm ${OUTCOMES[outcome].bar} ${outcome === "running" ? "animate-pulse" : ""} hover:opacity-70"></a>`;
    })
    .join("");

  return `<div class="flex gap-[2px]">${padding}${squares}</div>`;
}

function renderPipelines(entries) {
  if (entries.length === 0) {
    return `<div class="md:col-span-2 lg:col-span-3 rounded-md border border-dashed border-gray-300 dark:border-neutral-600 p-6 text-sm text-gray-600 dark:text-gray-300 space-y-3">
        <p>No pipelines in this namespace yet. Register one from a pipeline config directory:</p>
        ${commandBlock(`gofer up --deploy <path>${namespaceFlag()}`)}
      </div>`;
  }

  // Most recently active first; pipelines that have never run go last.
  const sorted = [...entries].sort(
    (a, b) => (b.runs[0]?.started || 0) - (a.runs[0]?.started || 0) || a.pipeline.pipeline_id.localeCompare(b.pipeline.pipeline_id)
  );

  return sorted
    .map((entry) => {
      const id = entry.pipeline.pipeline_id;
      const name = entry.pipeline.name;
      const disabled = String(entry.pipeline.state).toLowerCase() === "disabled";
      const latest = entry.runs[0];

      const finished = entry.runs.filter((run) => !["running", "cancelled"].includes(runOutcome(run)));
      const succeeded = finished.filter((run) => runOutcome(run) === "successful").length;
      const rate = finished.length ? `${Math.round((succeeded / finished.length) * 100)}%` : "-";
      const typical = finished.length ? compactDuration(median(finished.map(runDurationMs))) : "-";

      const stat = (label, value, title = "") =>
        `<div ${title ? `title="${escapeHtml(title)}"` : ""}><dt class="text-xs text-gray-500 dark:text-gray-400">${label}</dt><dd class="mt-0.5 text-sm font-medium tabular-nums">${value}</dd></div>`;

      return `<article class="flex flex-col rounded-md ring-1 ring-gray-200 dark:ring-neutral-700 p-4 ${disabled ? "opacity-60" : ""}">
          <div class="flex items-start justify-between gap-2">
            <div class="min-w-0">
              <h3 class="truncate font-medium">${escapeHtml(name || id)}</h3>
              ${name && name !== id ? `<p class="truncate font-mono text-xs text-gray-500 dark:text-gray-400">${escapeHtml(id)}</p>` : ""}
            </div>
            ${disabled ? `<span class="shrink-0 rounded-full bg-gray-100 dark:bg-neutral-700 px-2 py-0.5 text-xs text-gray-600 dark:text-gray-300">disabled</span>` : ""}
            ${latest && runOutcome(latest) === "running" ? `<span class="shrink-0 flex items-center gap-1.5 text-xs text-gray-600 dark:text-gray-300"><span class="size-2 rounded-full bg-yellow-400 animate-pulse"></span>running</span>` : ""}
          </div>
          ${entry.pipeline.description ? `<p class="mt-2 line-clamp-2 text-sm text-gray-600 dark:text-gray-300" title="${escapeHtml(entry.pipeline.description)}">${escapeHtml(entry.pipeline.description)}</p>` : ""}
          <div class="mt-4 flex-1 flex flex-col justify-end space-y-4">
            ${renderHistory(entry.runs)}
            <dl class="grid grid-cols-3 gap-2">
              ${stat("Last run", latest ? `<a href="${escapeHtml(runDetailsLink(latest))}" class="hover:text-emerald-600">${relativeTime(latest.started)}</a>` : "Never", latest ? formatTimestamp(latest.started) : "")}
              ${stat("Success", rate, "Of finished runs shown, not counting cancelled")}
              ${stat("Typical", typical, "Median run time of finished runs shown")}
            </dl>
            ${commandBlock(`gofer run list ${id}${namespaceFlag()}`)}
          </div>
        </article>`;
    })
    .join("");
}

function renderRunTable(runs) {
  if (runs.length === 0) {
    return `<tr><td colspan="6" class="px-4 py-8 text-center text-gray-500 dark:text-gray-400">No runs yet.</td></tr>`;
  }

  return runs
    .map((run) => {
      const link = escapeHtml(runDetailsLink(run));
      const badge = runOutcome(run) === "running" ? statusBadge(run.state) : statusBadge(run.status);
      const reason = run.status_reason ? `${run.status_reason.reason}: ${run.status_reason.description}` : "";

      return `<tr data-href="${link}" class="cursor-pointer hover:bg-gray-50 dark:hover:bg-neutral-700/40">
          <td class="px-4 py-2.5 whitespace-nowrap">
            <a href="${link}" class="font-medium hover:text-emerald-600">${escapeHtml(run.pipeline_id)}</a>
            <span class="ml-1 text-gray-500 dark:text-gray-400">#${escapeHtml(run.run_id)}</span>
          </td>
          <td class="px-4 py-2.5" ${reason ? `title="${escapeHtml(reason)}"` : ""}>${badge}</td>
          <td class="px-4 py-2.5 whitespace-nowrap" title="${formatTimestamp(run.started)}">${relativeTime(run.started)}</td>
          <td class="px-4 py-2.5 whitespace-nowrap tabular-nums">${run.started ? compactDuration(runDurationMs(run)) : "-"}</td>
          <td class="px-4 py-2.5 whitespace-nowrap" title="Token ID: ${escapeHtml(run.initiator.id)}">${escapeHtml(run.initiator.user)}</td>
          <td class="px-4 py-2.5 text-right text-gray-400">›</td>
        </tr>`;
    })
    .join("");
}

// ---------------------------------------------------------------------------------------------------------------
// Interaction
// ---------------------------------------------------------------------------------------------------------------

document.addEventListener("click", (event) => {
  const filter = event.target.closest("[data-run-filter]");
  if (filter) {
    runFilter = filter.dataset.runFilter;
    renderRecentRuns(lastEntries.flatMap((entry) => entry.runs));
    return;
  }

  const row = event.target.closest("tr[data-href]");
  if (row && !event.target.closest("a, button")) {
    window.location.href = row.dataset.href;
  }
});

document.addEventListener("change", (event) => {
  if (event.target.id === "namespace-select") {
    const params = new URLSearchParams({ namespace: event.target.value });
    window.location.search = params.toString();
  }
});

document.addEventListener("DOMContentLoaded", async function () {
  // Signing in or out can change which namespaces and runs are visible, so reload everything.
  const reload = () => {
    if (!session) runFilter = "all";
    loadNamespaces();
    loadDashboard();
  };

  // The session decides whether "Started by me" is offered, so the header goes first.
  await setupHeader({ active: "runs", onSessionChange: reload });
  loadNamespaces();
  await loadDashboard();

  setInterval(loadDashboard, 5000);
});
