// The run details page. It shows roughly what `gofer run get` and `gofer run debug` print, plus a dependency graph.
// Most of the "what went wrong" logic is ported from src/cli/run/debug.rs so the CLI and the web UI agree.

const params = new URLSearchParams(window.location.search);
const target = {
  namespace: params.get("namespace") || "default",
  pipeline: params.get("pipeline") || "",
  run: params.get("run") || "",
};

// Which task rows the user has expanded, so a refresh doesn't collapse them.
const expandedTasks = new Set();

let pollTimer = null;

// ---------------------------------------------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------------------------------------------

async function loadRun() {
  if (!target.pipeline || !/^[1-9]\d*$/.test(target.run)) {
    showError("This link is missing a pipeline or run id.");
    stopPolling();
    return;
  }

  const base = `/api/namespaces/${encodeURIComponent(target.namespace)}/pipelines/${encodeURIComponent(target.pipeline)}/runs/${target.run}`;
  const [runResp, tasksResp] = await Promise.all([apiFetch(base), apiFetch(`${base}/tasks`)]);

  if (!runResp.ok) {
    if (runResp.status === 404) {
      showError(`Run #${target.run} for pipeline ${target.pipeline} in namespace ${target.namespace} doesn't exist.`);
      stopPolling();
    } else if (runResp.status === 401 || runResp.status === 403) {
      showError("You need a token with access to this pipeline to view this run. Use Sign in at the top right.");
    } else {
      showError(`Could not load this run (status ${runResp.status || "network error"}). Retrying.`);
    }
    return;
  }

  const run = runResp.data.run;
  const tasks = tasksResp.ok ? tasksResp.data.task_executions : null;

  render(run, tasks, tasksResp.status);

  // Once the run is complete nothing on the page can change anymore. We keep polling if we couldn't read the tasks
  // so that entering a token fills them in.
  if (lower(run.state) === "complete" && tasks !== null) {
    stopPolling();
  }
}

function startPolling() {
  stopPolling();
  pollTimer = setInterval(loadRun, 5000);
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

function showError(message) {
  const el = document.getElementById("run-error");
  el.textContent = message;
  el.classList.remove("hidden");
  document.getElementById("run-content").classList.add("hidden");
}

function render(run, tasks, tasksStatus) {
  document.getElementById("run-error").classList.add("hidden");
  document.getElementById("run-content").classList.remove("hidden");
  document.title = `Gofer │ ${run.pipeline_id} #${run.run_id}`;

  setHtml("run-title", renderTitle(run));
  setHtml("run-reason", renderRunReason(run));
  setHtml("run-summary", renderSummary(run));
  setHtml("run-cli-tips", renderRunCliTips(run, tasks));

  const variablesSection = document.getElementById("run-variables-section");
  variablesSection.classList.toggle("hidden", run.variables.length === 0);
  setHtml("run-variables", renderVariables(run.variables));

  const notice = document.getElementById("tasks-notice");
  const sections = document.getElementById("task-sections");

  if (tasks === null) {
    notice.textContent =
      tasksStatus === 401 || tasksStatus === 403
        ? isTokenSaved()
          ? "Your token doesn't have access to this pipeline's tasks, so the task graph, timeline, and problems are hidden."
          : "Sign in at the top right to see this run's tasks, task graph, timeline, and what went wrong."
        : `Could not load this run's tasks (status ${tasksStatus || "network error"}).`;
    notice.classList.remove("hidden");
    sections.classList.add("hidden");
    return;
  }

  notice.classList.add("hidden");
  sections.classList.remove("hidden");

  const ordered = sortByRunOrder(tasks);

  setHtml("graph-legend", renderGraphLegend());
  setHtml("task-graph", renderGraph(tasks));
  setHtml("task-timeline", renderTimeline(run, ordered));
  const problems = renderProblems(ordered);
  document.getElementById("problems-section").classList.toggle("hidden", problems === "");
  setHtml("task-problems", problems);
  setHtml("task-table", renderTaskTable(ordered));
}

// ---------------------------------------------------------------------------------------------------------------
// Logic ported from src/cli/run/debug.rs
// ---------------------------------------------------------------------------------------------------------------

function lower(value) {
  return String(value ?? "").toLowerCase();
}

// How many levels of dependencies sit above a task. `limit` keeps a malformed cycle from looping forever.
function dependencyDepth(taskId, deps, limit) {
  if (limit === 0) {
    return 0;
  }

  let depth = 0;
  for (const parent of Object.keys(deps[taskId] || {})) {
    depth = Math.max(depth, 1 + dependencyDepth(parent, deps, limit - 1));
  }
  return depth;
}

function dependencyDepths(tasks) {
  const deps = {};
  for (const task of tasks) {
    deps[task.task_id] = task.task.depends_on || {};
  }

  const depths = {};
  for (const task of tasks) {
    depths[task.task_id] = dependencyDepth(task.task_id, deps, tasks.length);
  }
  return depths;
}

// Sorts tasks in the order they ran. Tasks that never started go last, ordered by how deep they sit in the
// dependency chain so a skipped task always comes after the skipped task it was waiting on.
function sortByRunOrder(tasks) {
  const depths = dependencyDepths(tasks);

  return [...tasks].sort((a, b) => {
    const keyA = [a.started === 0 ? 1 : 0, a.started, depths[a.task_id]];
    const keyB = [b.started === 0 ? 1 : 0, b.started, depths[b.task_id]];
    for (let i = 0; i < keyA.length; i++) {
      if (keyA[i] !== keyB[i]) return keyA[i] - keyB[i];
    }
    return a.task_id.localeCompare(b.task_id);
  });
}

function isFailure(task) {
  return lower(task.state) === "complete" && ["failed", "unknown"].includes(lower(task.status));
}

function isProblem(task) {
  return isFailure(task) || ["cancelled", "skipped"].includes(lower(task.status));
}

// Failures first since those are usually the root cause, then cancellations, then skips.
function problemRank(task) {
  switch (lower(task.status)) {
    case "failed":
    case "unknown":
      return 0;
    case "cancelled":
      return 1;
    default:
      return 2;
  }
}

function dependencyMet(parent, required) {
  switch (lower(required)) {
    case "any":
      return lower(parent.state) === "complete";
    case "success":
      return lower(parent.status) === "successful";
    case "failure":
      return lower(parent.status) === "failed";
    default:
      return true;
  }
}

function outcome(task) {
  switch (lower(task.status)) {
    case "successful":
      return "succeeded";
    case "failed":
      return "failed";
    case "cancelled":
      return "was cancelled";
    case "skipped":
      return "was skipped";
    default:
      return "ended in an unknown state";
  }
}

const NEEDS = { any: "to finish", success: "to succeed", failure: "to fail" };

function sortedDependencies(task) {
  return Object.entries(task.task.depends_on || {}).sort(([a], [b]) => a.localeCompare(b));
}

// Walks up from a skipped task to the first unmet dependency that wasn't itself skipped. `depth` keeps a malformed
// dependency cycle from looping forever.
function rootCause(task, byId, depth) {
  if (depth === 0) {
    return null;
  }

  for (const [parentId, required] of sortedDependencies(task)) {
    const parent = byId[parentId];
    if (!parent || dependencyMet(parent, required)) {
      continue;
    }

    if (lower(parent.status) === "skipped") {
      return rootCause(parent, byId, depth - 1);
    }

    return parent;
  }

  return null;
}

// Explains which of a skipped task's dependencies weren't met, following skipped parents up to the task that
// actually caused it so the user doesn't have to.
function unmetDependencies(task, byId) {
  const reasons = [];

  for (const [parentId, required] of sortedDependencies(task)) {
    const parent = byId[parentId];
    if (!parent || dependencyMet(parent, required)) {
      continue;
    }

    const needs = NEEDS[lower(required)];
    if (!needs) {
      continue;
    }

    let reason = `needs ${taskName(parentId)} ${needs}, but it ${outcome(parent)}`;

    if (lower(parent.status) === "skipped") {
      const root = rootCause(parent, byId, Object.keys(byId).length);
      if (root) {
        reason += ` because ${taskName(root.task_id)} ${outcome(root)}`;
      }
    }

    reasons.push(reason);
  }

  return reasons;
}

// A short offset from the start of the run, like "48s", "1m52s", or "1h05m".
function offset(millis) {
  const secs = Math.floor(Math.max(millis, 0) / 1000);
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m${String(secs % 60).padStart(2, "0")}s`;
  return `${Math.floor(secs / 3600)}h${String(Math.floor((secs % 3600) / 60)).padStart(2, "0")}m`;
}

// A task's status in a word or two.
function statusText(task) {
  if (lower(task.state) !== "complete") {
    return lower(task.state);
  }
  if (lower(task.status) === "failed" && task.exit_code !== null && task.exit_code !== undefined) {
    return `failed (exit ${task.exit_code})`;
  }
  return lower(task.status);
}

// Collapses a task's state and status into one of a few color families shared by the graph and the timeline.
function taskTone(task) {
  if (lower(task.state) !== "complete") {
    return task.started === 0 ? "waiting" : "running";
  }
  switch (lower(task.status)) {
    case "successful":
      return "success";
    case "failed":
      return "failed";
    case "unknown":
      return "unknown";
    default:
      return "muted";
  }
}

// "ran for 2 minutes" once the run is done, "has been running for 2 minutes" while it's still going.
function runDuration(run) {
  if (run.started === 0) {
    return "hasn't started yet";
  }
  const elapsed = humanizeDuration(run.started, run.ended || Date.now());
  return lower(run.state) === "complete" ? `ran for ${elapsed}` : `has been running for ${elapsed}`;
}

function taskDuration(task) {
  if (task.started === 0) {
    return "-";
  }
  const elapsed = humanizeDuration(task.started, task.ended || Date.now());
  switch (lower(task.state)) {
    case "running":
      return `Running for ${elapsed}`;
    case "waiting":
    case "processing":
      return `Waiting for ${elapsed}`;
    default:
      return `Lasted ${elapsed}`;
  }
}

// Mirrors `dependencies()` in src/cli/mod.rs: "After task build has succeeded." and friends.
function startsAfter(task) {
  const groups = { any: [], success: [], failure: [] };
  for (const [parentId, required] of sortedDependencies(task)) {
    groups[lower(required)]?.push(parentId);
  }

  const phrase = (ids, verb) => {
    if (ids.length === 0) return null;
    const names = ids.map(taskName).join(", ");
    return ids.length === 1 ? `After task ${names} has ${verb}.` : `After tasks ${names} have ${verb}.`;
  };

  const lines = [phrase(groups.any, "finished"), phrase(groups.success, "succeeded"), phrase(groups.failure, "failed")].filter(Boolean);
  return lines.length ? lines.join("<br>") : `<span class="text-gray-500 dark:text-gray-400">Immediately</span>`;
}

// ---------------------------------------------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------------------------------------------

function taskName(taskId) {
  return `<span class="font-mono text-sky-700 dark:text-sky-300">${escapeHtml(taskId)}</span>`;
}

function namespaceFlag() {
  return target.namespace === "default" ? "" : ` --namespace ${target.namespace}`;
}

function renderVariables(variables) {
  if (!variables || variables.length === 0) {
    return `<p class="text-sm text-gray-500 dark:text-gray-400">None</p>`;
  }

  const rows = variables
    .map(
      (variable) => `<tr>
        <td class="py-1.5 pr-4 font-mono text-xs align-top">${escapeHtml(variable.key)}</td>
        <td class="py-1.5 pr-4 font-mono text-xs break-all">${escapeHtml(variable.value)}</td>
        <td class="py-1.5 text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap align-top">${escapeHtml(variable.source)}</td>
      </tr>`
    )
    .join("");

  return `<table class="w-full text-left">
      <thead><tr class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">
        <th class="pb-2 pr-4 font-medium">Key</th><th class="pb-2 pr-4 font-medium">Value</th><th class="pb-2 font-medium">Source</th>
      </tr></thead>
      <tbody class="divide-y divide-gray-100 dark:divide-neutral-700">${rows}</tbody>
    </table>`;
}

// ---------------------------------------------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------------------------------------------

function renderTitle(run) {
  return `<nav class="text-sm text-gray-500 dark:text-gray-400">
      <a href="/" class="hover:text-emerald-600">Runs</a>
      <span class="mx-1">/</span>
      <span>${escapeHtml(run.namespace_id)}</span>
      <span class="mx-1">/</span>
      <span>${escapeHtml(run.pipeline_id)}</span>
    </nav>
    <div class="mt-2 flex flex-wrap items-center gap-3">
      <h1 class="text-3xl font-semibold">Run <span class="text-emerald-500">#${escapeHtml(run.run_id)}</span></h1>
      <span class="rounded-full bg-gray-100 dark:bg-neutral-700 px-2 py-0.5 text-xs text-gray-600 dark:text-gray-300" title="Pipeline config version">config v${escapeHtml(run.pipeline_config_version)}</span>
      <span class="ml-auto flex gap-2">${statusBadge(run.state)}${statusBadge(run.status)}</span>
    </div>`;
}

function renderRunReason(run) {
  if (!run.status_reason) {
    return "";
  }

  const cancelled = lower(run.status) === "cancelled";
  const colors = cancelled
    ? "border-slate-300 dark:border-slate-600 bg-slate-50 dark:bg-slate-800/50 text-slate-800 dark:text-slate-200"
    : "border-red-300 dark:border-red-800 bg-red-50 dark:bg-red-950/40 text-red-800 dark:text-red-200";

  return `<div class="rounded-md border ${colors} p-4 text-sm">
      <span class="font-semibold">${cancelled ? "Cancelled" : "Failure"}: ${escapeHtml(run.status_reason.reason)}</span>
      <p class="mt-1">${escapeHtml(run.status_reason.description)}</p>
    </div>`;
}

function renderSummary(run) {
  const cell = (label, value, title = "") =>
    `<div class="bg-white dark:bg-neutral-800 px-4 py-3" ${title ? `title="${escapeHtml(title)}"` : ""}>
      <dt class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">${label}</dt>
      <dd class="mt-1 text-sm font-medium">${value}</dd>
    </div>`;

  const cells = [
    cell("Initiated by", escapeHtml(run.initiator.user), `Token ID: ${run.initiator.id}`),
    cell("Started", run.started ? formatTimestamp(run.started) : "Not yet"),
    cell("Ended", run.ended ? formatTimestamp(run.ended) : "-"),
    cell("Duration", escapeHtml(runDuration(run))),
    cell("Objects expired", run.store_objects_expired ? "Yes" : "No"),
    cell("Injected token", run.token_id ? `<span class="font-mono text-xs break-all">${escapeHtml(run.token_id)}</span>` : "None"),
  ];

  return cells.join("");
}

// Tailwind needs to see full class names in the source, so the tone palettes are spelled out instead of built.
const TONES = {
  success: {
    node: "fill-emerald-50 stroke-emerald-500 dark:fill-emerald-950",
    dot: "fill-emerald-500",
    bar: "bg-emerald-500",
  },
  failed: {
    node: "fill-red-50 stroke-red-500 dark:fill-red-950",
    dot: "fill-red-500",
    bar: "bg-red-500",
  },
  running: {
    node: "fill-yellow-50 stroke-yellow-500 dark:fill-yellow-950",
    dot: "fill-yellow-500 animate-pulse",
    bar: "bg-yellow-400 animate-pulse",
  },
  waiting: {
    node: "fill-white stroke-gray-300 dark:fill-neutral-800 dark:stroke-neutral-600",
    dot: "fill-gray-300 dark:fill-neutral-500",
    bar: "bg-gray-300",
  },
  muted: {
    node: "fill-gray-100 stroke-gray-400 dark:fill-neutral-800 dark:stroke-neutral-500",
    dot: "fill-gray-400",
    bar: "bg-gray-400",
  },
  unknown: {
    node: "fill-purple-50 stroke-purple-500 dark:fill-purple-950",
    dot: "fill-purple-500",
    bar: "bg-purple-500",
  },
};

// Edge styles keyed by the parent status a task requires.
const EDGES = {
  success: { stroke: "stroke-emerald-500", marker: "fill-emerald-500", dash: "", label: "needs success" },
  failure: { stroke: "stroke-red-500", marker: "fill-red-500", dash: "6 4", label: "needs failure" },
  any: { stroke: "stroke-gray-400", marker: "fill-gray-400", dash: "2 4", label: "waiting on complete" },
};

function renderGraphLegend() {
  return Object.values(EDGES)
    .map(
      (edge) => `<span class="flex items-center gap-1.5">
        <svg width="24" height="6" aria-hidden="true"><line x1="0" y1="3" x2="24" y2="3" stroke-width="2" class="${edge.stroke}" ${edge.dash ? `stroke-dasharray="${edge.dash}"` : ""}/></svg>
        ${edge.label}
      </span>`
    )
    .join("");
}

const NODE_WIDTH = 190;
const NODE_HEIGHT = 52;
const COLUMN_GAP = 90;
const ROW_GAP = 20;
const GRAPH_PADDING = 16;

// Lays tasks out left to right by dependency depth, so every edge points rightward from a parent to its child.
function renderGraph(tasks) {
  if (tasks.length === 0) {
    return `<p class="p-4 text-sm text-gray-500 dark:text-gray-400">This run has no tasks.</p>`;
  }

  const depths = dependencyDepths(tasks);
  const columns = [];
  for (const task of [...tasks].sort((a, b) => a.task_id.localeCompare(b.task_id))) {
    (columns[depths[task.task_id]] ||= []).push(task);
  }
  const filled = columns.filter(Boolean);

  const tallest = Math.max(...filled.map((column) => column.length));
  const width = GRAPH_PADDING * 2 + filled.length * NODE_WIDTH + (filled.length - 1) * COLUMN_GAP;
  const height = GRAPH_PADDING * 2 + tallest * NODE_HEIGHT + (tallest - 1) * ROW_GAP;

  const positions = {};
  filled.forEach((column, col) => {
    // Center shorter columns vertically so the graph reads as a flow rather than a staircase.
    const columnHeight = column.length * NODE_HEIGHT + (column.length - 1) * ROW_GAP;
    const top = GRAPH_PADDING + (height - GRAPH_PADDING * 2 - columnHeight) / 2;
    column.forEach((task, row) => {
      positions[task.task_id] = {
        x: GRAPH_PADDING + col * (NODE_WIDTH + COLUMN_GAP),
        y: top + row * (NODE_HEIGHT + ROW_GAP),
      };
    });
  });

  const markers = Object.entries(EDGES)
    .map(
      ([kind, edge]) =>
        `<marker id="arrow-${kind}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
          <path d="M 0 0 L 10 5 L 0 10 z" class="${edge.marker}"/>
        </marker>`
    )
    .join("");

  const edges = [];
  for (const task of tasks) {
    const to = positions[task.task_id];
    for (const [parentId, required] of sortedDependencies(task)) {
      const from = positions[parentId];
      const kind = lower(required);
      const edge = EDGES[kind];
      if (!from || !edge) continue;

      const x1 = from.x + NODE_WIDTH;
      const y1 = from.y + NODE_HEIGHT / 2;
      const x2 = to.x - 2;
      const y2 = to.y + NODE_HEIGHT / 2;
      const bend = Math.max((x2 - x1) / 2, 30);

      edges.push(`<path d="M ${x1} ${y1} C ${x1 + bend} ${y1}, ${x2 - bend} ${y2}, ${x2} ${y2}"
          fill="none" stroke-width="2" class="${edge.stroke}" ${edge.dash ? `stroke-dasharray="${edge.dash}"` : ""}
          marker-end="url(#arrow-${kind})">
          <title>${escapeHtml(task.task_id)} needs ${escapeHtml(parentId)} ${NEEDS[kind]}</title>
        </path>`);
    }
  }

  const nodes = tasks.map((task) => {
    const { x, y } = positions[task.task_id];
    const tone = TONES[taskTone(task)];
    const label = task.task_id.length > 22 ? task.task_id.slice(0, 21) + "…" : task.task_id;
    const detail = task.started ? `${statusText(task)} · ${offset((task.ended || Date.now()) - task.started)}` : statusText(task);

    return `<g data-task="${escapeHtml(task.task_id)}" class="cursor-pointer group" tabindex="0" role="button">
        <title>${escapeHtml(task.task_id)}${task.task.description ? ": " + escapeHtml(task.task.description) : ""}</title>
        <rect x="${x}" y="${y}" width="${NODE_WIDTH}" height="${NODE_HEIGHT}" rx="8" stroke-width="1.5"
          class="${tone.node} group-hover:stroke-[2.5px] transition-all"/>
        <circle cx="${x + 16}" cy="${y + NODE_HEIGHT / 2}" r="5" class="${tone.dot}"/>
        <text x="${x + 30}" y="${y + 22}" class="fill-gray-900 dark:fill-gray-100 font-mono text-[13px]">${escapeHtml(label)}</text>
        <text x="${x + 30}" y="${y + 39}" class="fill-gray-500 dark:fill-gray-400 text-[11px]">${escapeHtml(detail)}</text>
      </g>`;
  });

  return `<svg width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" class="block" role="img" aria-label="Task dependency graph">
      <defs>${markers}</defs>
      ${edges.join("")}
      ${nodes.join("")}
    </svg>`;
}

function renderTimeline(run, tasks) {
  if (tasks.length === 0) {
    return `<p class="text-sm text-gray-500 dark:text-gray-400">None</p>`;
  }

  const now = Date.now();
  const created = tasks.map((task) => task.created).filter(Boolean);
  const runStart = run.started || (created.length ? Math.min(...created) : now);
  const runEnd = run.ended || now;
  const span = Math.max(runEnd - runStart, 1);
  const percent = (time) => Math.min(Math.max(((time - runStart) / span) * 100, 0), 100);

  const rows = tasks.map((task) => {
    let bar = "";
    let times = "not started";

    if (task.started !== 0) {
      const end = task.ended || now;
      const waitFrom = percent(Math.max(task.created, runStart));
      const runFrom = percent(task.started);
      // Very short tasks still get a sliver so they don't vanish.
      const runWidth = Math.max(percent(end) - runFrom, 0.75);
      const tone = TONES[taskTone(task)];

      bar = `<div class="absolute inset-y-0 rounded-sm bg-gray-200 dark:bg-neutral-600"
            style="left:${waitFrom}%;width:${Math.max(runFrom - waitFrom, 0)}%" title="waiting"></div>
          <div class="absolute inset-y-0 rounded-sm ${tone.bar}"
            style="left:${Math.min(runFrom, 100 - runWidth)}%;width:${runWidth}%" title="running"></div>`;
      times = `${offset(task.started - runStart)} → ${task.ended ? offset(task.ended - runStart) : "now"}`;
    }

    return `<button type="button" data-task="${escapeHtml(task.task_id)}" class="truncate text-left font-mono text-xs text-sky-700 dark:text-sky-300 py-1 hover:underline">${escapeHtml(task.task_id)}</button>
        <span class="relative my-1 h-4 rounded-sm bg-gray-50 dark:bg-neutral-900/60">${bar}</span>
        <span class="whitespace-nowrap font-mono text-xs text-gray-500 dark:text-gray-400 py-1">${times}</span>
        <span class="whitespace-nowrap text-xs py-1">${escapeHtml(statusText(task))}</span>`;
  });

  return `<div class="grid grid-cols-[minmax(6rem,12rem)_1fr_auto_auto] items-center gap-x-4">
      ${rows.join("")}
      <span></span>
      <span class="flex justify-between border-t border-gray-200 dark:border-neutral-700 pt-1 font-mono text-xs text-gray-400">
        <span>0s</span><span>${offset(span / 2)}</span><span>${offset(span)}</span>
      </span>
      <span></span><span></span>
    </div>
    <div class="mt-3 flex gap-4 text-xs text-gray-500 dark:text-gray-400">
      <span class="flex items-center gap-1.5"><span class="inline-block h-2.5 w-4 rounded-sm bg-gray-200 dark:bg-neutral-600"></span>waiting</span>
      <span class="flex items-center gap-1.5"><span class="inline-block h-2.5 w-4 rounded-sm bg-emerald-500"></span>running</span>
    </div>`;
}

function labeled(label, value) {
  return `<div class="grid grid-cols-[5rem_1fr] gap-2">
      <dt class="text-gray-500 dark:text-gray-400">${label}</dt>
      <dd class="min-w-0 break-words">${value}</dd>
    </div>`;
}

function renderProblems(tasks) {
  const byId = Object.fromEntries(tasks.map((task) => [task.task_id, task]));
  const problems = tasks.filter(isProblem).sort((a, b) => problemRank(a) - problemRank(b));

  // The section is hidden entirely when this comes back empty.
  if (problems.length === 0) {
    return "";
  }

  const cards = problems.map((task) => {
    const status = lower(task.status);
    let icon, headline, accent, body;

    if (status === "skipped") {
      icon = "↷";
      headline = "was skipped";
      accent = "border-l-gray-400";
      let reasons = unmetDependencies(task, byId);
      if (reasons.length === 0 && task.status_reason) {
        reasons = [escapeHtml(task.status_reason.description)];
      }
      body = reasons.length ? labeled("Reason", reasons.join("<br>")) : "";
    } else if (status === "cancelled") {
      icon = "⊘";
      headline = "was cancelled";
      accent = "border-l-slate-500";
      body = task.status_reason ? labeled("Reason", escapeHtml(task.status_reason.description)) : "";
    } else {
      icon = "✗";
      accent = "border-l-red-500";
      headline =
        status === "failed"
          ? task.exit_code !== null && task.exit_code !== undefined
            ? `failed with exit code ${task.exit_code}`
            : "failed"
          : "ended in an unknown state";

      const parts = [];
      if (task.status_reason) {
        parts.push(labeled("Reason", `<span class="font-medium">${escapeHtml(task.status_reason.reason)}</span>: ${escapeHtml(task.status_reason.description)}`));
      }
      parts.push(labeled("Image", `<span class="font-mono text-xs">${escapeHtml(task.task.image)}</span>`));
      if (task.image_digest) {
        parts.push(labeled("Digest", `<span class="font-mono text-xs break-all">${escapeHtml(task.image_digest)}</span>`));
      }

      let output;
      if (task.started === 0) {
        output = `<span class="text-gray-500 dark:text-gray-400">The task never started, so there is no output.</span>`;
      } else if (task.logs_removed || task.logs_expired) {
        output = `<span class="text-gray-500 dark:text-gray-400">The logs have been removed.</span>`;
      } else {
        output = commandBlock(`gofer task logs ${task.pipeline_id} ${task.run_id} ${task.task_id}${namespaceFlag()}`);
      }
      parts.push(labeled("Output", output));
      body = parts.join("");
    }

    return `<div class="rounded-md border border-gray-200 dark:border-neutral-700 border-l-4 ${accent} p-4">
        <p class="text-sm"><span class="mr-1">${icon}</span>${taskName(task.task_id)} ${headline}</p>
        <dl class="mt-3 space-y-2 text-sm">${body}</dl>
      </div>`;
  });

  return cards.join("");
}

// The terminal equivalents of what this page shows, so people can pick up the CLI as they go.
function renderRunCliTips(run, tasks) {
  const tip = (text, command) =>
    `<div class="grid gap-1 md:grid-cols-[16rem_1fr] md:items-center">
      <p class="text-sm text-gray-600 dark:text-gray-300">${text}</p>
      ${commandBlock(command)}
    </div>`;

  const tips = [tip("See this run from your terminal:", `gofer run debug ${run.pipeline_id} ${run.run_id}${namespaceFlag()}`)];

  const running = (tasks || []).find((task) => lower(task.state) === "running");
  if (running) {
    tips.push(
      tip(
        `Open a shell in ${taskName(running.task_id)} while it's running:`,
        `gofer task attach ${run.pipeline_id} ${run.run_id} ${running.task_id}${namespaceFlag()}`
      )
    );
  }

  return tips.join("");
}

// Per-task commands shown in the expanded row of the tasks table.
function taskCliTips(task) {
  const commands = [];
  const logsAvailable = task.started !== 0 && !task.logs_removed && !task.logs_expired;

  if (logsAvailable) {
    commands.push(commandBlock(`gofer task logs ${task.pipeline_id} ${task.run_id} ${task.task_id}${namespaceFlag()}`));
  }
  if (lower(task.state) === "running") {
    commands.push(commandBlock(`gofer task attach ${task.pipeline_id} ${task.run_id} ${task.task_id}${namespaceFlag()}`));
  }

  if (commands.length === 0) {
    return "";
  }

  return `<div class="md:col-span-2 space-y-2">
      <h3 class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">From the CLI</h3>
      ${commands.join("")}
    </div>`;
}

function formatArgs(args) {
  if (!args || args.length === 0) {
    return `<span class="text-gray-500 dark:text-gray-400">Image default</span>`;
  }
  const quoted = args.map((arg) => (/[\s"']/.test(arg) ? JSON.stringify(arg) : arg)).join(" ");
  return `<code class="font-mono text-xs whitespace-pre-wrap break-all">${escapeHtml(quoted)}</code>`;
}

function renderTaskDetails(task) {
  const yesNo = (value) => (value ? "Yes" : "No");
  const logs = task.logs_removed ? "Removed" : task.logs_expired ? "Expired" : "Available";

  // Once a task starts, its execution carries the variables that were actually injected. Before then the best we
  // can show is what the pipeline config asks for.
  const started = task.variables && task.variables.length > 0;
  const variables = started ? task.variables : task.task.variables;

  return `<div class="grid gap-6 md:grid-cols-2">
      <dl class="space-y-2 text-sm">
        ${task.task.description ? labeled("About", escapeHtml(task.task.description)) : ""}
        ${labeled("Image", `<span class="font-mono text-xs break-all">${escapeHtml(task.task.image)}</span>`)}
        ${task.image_digest ? labeled("Digest", `<span class="font-mono text-xs break-all">${escapeHtml(task.image_digest)}</span>`) : ""}
        ${labeled("Entrypoint", formatArgs(task.task.entrypoint))}
        ${labeled("Command", formatArgs(task.task.command))}
        ${labeled("Exit code", task.exit_code ?? "-")}
        ${task.status_reason ? labeled("Reason", `${escapeHtml(task.status_reason.reason)}: ${escapeHtml(task.status_reason.description)}`) : ""}
        ${labeled("Created", formatTimestamp(task.created))}
        ${labeled("Ended", formatTimestamp(task.ended))}
        ${labeled("Logs", logs)}
        ${labeled("API token", yesNo(task.task.inject_api_token))}
        ${labeled("Always pull", yesNo(task.task.always_pull_newest_image))}
      </dl>
      <div>
        <h3 class="mb-2 text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">${started ? "Injected variables" : "Configured variables"}</h3>
        ${renderVariables(variables)}
      </div>
      ${taskCliTips(task)}
    </div>`;
}

function renderTaskTable(tasks) {
  return tasks
    .map((task) => {
      const expanded = expandedTasks.has(task.task_id);
      const id = escapeHtml(task.task_id);

      return `<tr id="task-${id}" data-task="${id}" data-toggle class="cursor-pointer hover:bg-gray-50 dark:hover:bg-neutral-700/40">
          <td class="px-4 py-2.5 whitespace-nowrap">
            <span class="mr-1 inline-block w-3 text-gray-400">${expanded ? "▾" : "▸"}</span>${taskName(task.task_id)}
          </td>
          <td class="px-4 py-2.5 whitespace-nowrap">${task.started ? formatTimestamp(task.started) : "Not yet"}</td>
          <td class="px-4 py-2.5 whitespace-nowrap">${taskDuration(task)}</td>
          <td class="px-4 py-2.5">${statusBadge(lower(task.state))}</td>
          <td class="px-4 py-2.5">${statusBadge(lower(task.status))}</td>
          <td class="px-4 py-2.5 text-xs">${startsAfter(task)}</td>
        </tr>
        <tr class="${expanded ? "" : "hidden"} bg-gray-50/60 dark:bg-neutral-900/30">
          <td colspan="6" class="px-6 py-4">${renderTaskDetails(task)}</td>
        </tr>`;
    })
    .join("");
}

// ---------------------------------------------------------------------------------------------------------------
// Interaction
// ---------------------------------------------------------------------------------------------------------------

function toggleTask(taskId, forceOpen = false) {
  if (forceOpen || !expandedTasks.has(taskId)) {
    expandedTasks.add(taskId);
  } else {
    expandedTasks.delete(taskId);
  }

  const row = document.querySelector(`#task-table tr[data-task="${CSS.escape(taskId)}"]`);
  if (!row) return;

  const open = expandedTasks.has(taskId);
  row.nextElementSibling.classList.toggle("hidden", !open);
  row.querySelector("td span").textContent = open ? "▾" : "▸";
  // The table's cached HTML no longer matches the DOM; clear it so the next refresh re-renders with this state.
  delete document.getElementById("task-table").dataset.rendered;
}

function jumpToTask(taskId) {
  toggleTask(taskId, true);
  const row = document.querySelector(`#task-table tr[data-task="${CSS.escape(taskId)}"]`);
  if (row) {
    row.scrollIntoView({ behavior: "smooth", block: "center" });
    row.classList.add("ring-2", "ring-emerald-400");
    setTimeout(() => row.classList.remove("ring-2", "ring-emerald-400"), 1500);
  }
}

document.addEventListener("click", (event) => {
  const row = event.target.closest("#task-table tr[data-toggle]");
  if (row) {
    toggleTask(row.dataset.task);
    return;
  }

  const link = event.target.closest("#task-graph [data-task], #task-timeline [data-task]");
  if (link) {
    jumpToTask(link.dataset.task);
  }
});

document.addEventListener("keydown", (event) => {
  const node = event.target.closest?.("#task-graph [data-task]");
  if (node && (event.key === "Enter" || event.key === " ")) {
    event.preventDefault();
    jumpToTask(node.dataset.task);
  }
});

document.addEventListener("DOMContentLoaded", async function () {
  // A new token can unlock this run's tasks, and polling may have stopped, so reload and restart it.
  setupHeader({
    onSessionChange: () => {
      loadRun();
      startPolling();
    },
  });

  startPolling();
  await loadRun();
});
