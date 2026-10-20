// The pipeline details page. It shows roughly what `gofer pipeline get` prints: recent runs, tasks, and
// subscriptions, plus a warning for anything that would stop the pipeline from running on its own.
//
// Anyone can see a default namespace pipeline and its runs, but configs and subscriptions need a token, so those
// sections say so instead of failing the whole page.

const RECENT_RUNS = 20;

const params = new URLSearchParams(window.location.search);
const target = {
  namespace: params.get("namespace") || "default",
  pipeline: params.get("pipeline") || "",
};

let pollTimer = null;

// ---------------------------------------------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------------------------------------------

async function loadPipeline() {
  if (!target.pipeline) {
    showError("This link is missing a pipeline id.");
    stopPolling();
    return;
  }

  const base = `/api/namespaces/${encodeURIComponent(target.namespace)}/pipelines/${encodeURIComponent(target.pipeline)}`;
  const [pipelineResp, configResp, subscriptionsResp, runsResp] = await Promise.all([
    apiFetch(base),
    apiFetch(`${base}/configs/0`),
    apiFetch(`${base}/subscriptions`),
    apiFetch(`${base}/runs?limit=${RECENT_RUNS}&reverse=true`),
  ]);

  if (!pipelineResp.ok) {
    if (pipelineResp.status === 404) {
      showError(`Pipeline ${target.pipeline} in namespace ${target.namespace} doesn't exist.`);
      stopPolling();
    } else if (pipelineResp.status === 401 || pipelineResp.status === 403) {
      showError("You need a token with access to this pipeline to view it. Use Sign in at the top right.");
    } else {
      showError(`Could not load this pipeline (status ${pipelineResp.status || "network error"}). Retrying.`);
    }
    return;
  }

  render({
    pipeline: pipelineResp.data.pipeline,
    config: configResp.ok ? configResp.data.config : null,
    configStatus: configResp.status,
    subscriptions: subscriptionsResp.ok ? subscriptionsResp.data.subscriptions : null,
    subscriptionsStatus: subscriptionsResp.status,
    runs: runsResp.ok ? runsResp.data.runs : [],
  });
}

function startPolling() {
  stopPolling();
  pollTimer = setInterval(loadPipeline, 5000);
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

function showError(message) {
  const el = document.getElementById("pipeline-error");
  el.textContent = message;
  el.classList.remove("hidden");
  document.getElementById("pipeline-content").classList.add("hidden");
}

function render(data) {
  document.getElementById("pipeline-error").classList.add("hidden");
  document.getElementById("pipeline-content").classList.remove("hidden");
  document.title = `Gofer │ ${data.pipeline.pipeline_id}`;

  setHtml("pipeline-title", renderTitle(data.pipeline, data.config));
  const attention = renderAttention(data.pipeline, data.subscriptions, data.runs);
  document.getElementById("attention-section").classList.toggle("hidden", attention === "");
  setHtml("attention", attention);
  setHtml("pipeline-summary", renderSummary(data.pipeline, data.config, data.runs));
  setHtml("pipeline-cli-tips", renderCliTips(data.pipeline));
  setHtml("run-list-body", renderRunTable(data.runs));
  setHtml("tasks", data.config ? renderTasks(data.config) : accessNotice(data.configStatus, "tasks"));
  setHtml(
    "subscriptions",
    data.subscriptions ? renderSubscriptions(data.subscriptions) : accessNotice(data.subscriptionsStatus, "subscriptions")
  );
}

// ---------------------------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------------------------

function lower(value) {
  return String(value ?? "").toLowerCase();
}

function namespaceFlag() {
  return target.namespace === "default" ? "" : ` --namespace ${target.namespace}`;
}

// Stands in for a section the visitor's token can't read.
function accessNotice(status, what) {
  const message =
    status === 401 || status === 403
      ? isTokenSaved()
        ? `Your token doesn't have access to this pipeline's ${what}.`
        : `Sign in at the top right to see this pipeline's ${what}.`
      : `Could not load this pipeline's ${what} (status ${status || "network error"}).`;
  return `<div class="rounded-md border border-dashed border-gray-300 dark:border-neutral-600 p-6 text-sm text-gray-600 dark:text-gray-300">${message}</div>`;
}

function emptyNotice(message, command) {
  return `<div class="rounded-md border border-dashed border-gray-300 dark:border-neutral-600 p-6 text-sm text-gray-600 dark:text-gray-300 space-y-3">
      <p>${message}</p>
      ${command ? commandBlock(command) : ""}
    </div>`;
}

// ---------------------------------------------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------------------------------------------

function renderTitle(pipeline, config) {
  const namespaceOverview = pipeline.namespace_id === "default" ? "/" : `/?namespace=${encodeURIComponent(pipeline.namespace_id)}`;
  const name = config?.name || pipeline.pipeline_id;

  return `<nav class="text-sm text-gray-500 dark:text-gray-400">
      <a href="/" class="hover:text-emerald-600">Overview</a>
      <span class="mx-1">/</span>
      <a href="${escapeHtml(namespaceOverview)}" class="hover:text-emerald-600">${escapeHtml(pipeline.namespace_id)}</a>
    </nav>
    <div class="mt-2 flex flex-wrap items-center gap-3">
      <h1 class="text-3xl font-semibold">${escapeHtml(name)}</h1>
      ${name !== pipeline.pipeline_id ? `<span class="font-mono text-sm text-gray-500 dark:text-gray-400">${escapeHtml(pipeline.pipeline_id)}</span>` : ""}
      ${config ? `<span class="rounded-full bg-gray-100 dark:bg-neutral-700 px-2 py-0.5 text-xs text-gray-600 dark:text-gray-300" title="Live pipeline config version">config v${escapeHtml(config.version)}</span>` : ""}
      <span class="ml-auto">${statusBadge(pipeline.state)}</span>
    </div>
    ${config?.description ? `<p class="mt-2 text-sm text-gray-600 dark:text-gray-300">${escapeHtml(config.description)}</p>` : ""}`;
}

// Everything wrong with the pipeline in one place: its latest run failing, the pipeline being disabled, or a
// subscription that's broken. A subscription in error does nothing at all, so this is often the only sign of why runs
// or notifications stopped.
function renderAttention(pipeline, subscriptions, runs) {
  const warning = (message, command) =>
    `<div class="rounded-md border border-red-300 dark:border-red-800 bg-red-50 dark:bg-red-950/40 p-4 text-sm text-red-800 dark:text-red-200 space-y-3">
      <p>${message}</p>
      ${command ? commandBlock(command) : ""}
    </div>`;

  const problems = [];

  // Same rule as the overview: only the most recent finished run counts, so a failure that's since been fixed by a
  // passing run doesn't linger here.
  const lastFinished = runs.find((run) => lower(run.state) === "complete");
  if (lastFinished && !["successful", "cancelled"].includes(lower(lastFinished.status))) {
    const reason = lastFinished.status_reason
      ? `<span class="mt-1 block text-red-700 dark:text-red-300">${escapeHtml(lastFinished.status_reason.description)}</span>`
      : "";
    problems.push(
      warning(
        `<span class="font-semibold">Run <a href="${escapeHtml(runDetailsLink(lastFinished))}" class="underline hover:no-underline">#${escapeHtml(lastFinished.run_id)}</a>, the latest to finish, failed</span>
        <span class="text-red-700 dark:text-red-300" title="${formatTimestamp(lastFinished.ended)}">${relativeTime(lastFinished.ended || lastFinished.created)}</span>.${reason}`,
        `gofer run debug ${pipeline.pipeline_id} ${lastFinished.run_id}${namespaceFlag()}`
      )
    );
  }

  if (lower(pipeline.state) === "disabled") {
    problems.push(
      warning(
        "This pipeline is disabled, so nothing can start a run, including its subscriptions.",
        `gofer pipeline enable ${pipeline.pipeline_id}${namespaceFlag()}`
      )
    );
  }

  for (const subscription of subscriptions || []) {
    if (lower(subscription.status) !== "error") continue;
    const reason = subscription.status_reason
      ? `<span class="mt-1 block text-red-700 dark:text-red-300">${escapeHtml(subscription.status_reason.description)}</span>`
      : "";
    problems.push(
      warning(
        `<span class="font-semibold">Subscription <span class="font-mono">${escapeHtml(subscription.subscription_id)}</span> won't do anything</span>
        until extension <span class="font-mono">${escapeHtml(subscription.extension_id)}</span> is running again.${reason}`
      )
    );
  }

  return problems.join("");
}

function renderSummary(pipeline, config, runs) {
  const cell = (label, value, title = "") =>
    `<div class="bg-white dark:bg-neutral-800 px-4 py-3" ${title ? `title="${escapeHtml(title)}"` : ""}>
      <dt class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">${label}</dt>
      <dd class="mt-1 text-sm font-medium">${value}</dd>
    </div>`;

  const latest = runs[0];
  const parallelism = config ? (config.parallelism === 0 ? "Server default" : `${config.parallelism} at a time`) : "-";

  return [
    cell("Created", formatTimestamp(pipeline.created)),
    cell("Last run", latest ? relativeTime(latest.created) : "Never", latest ? formatTimestamp(latest.created) : ""),
    cell("Config registered", config ? formatTimestamp(config.registered) : "-"),
    cell("Parallelism", parallelism, "How many runs can be in progress at once"),
  ].join("");
}

function renderCliTips(pipeline) {
  const tip = (text, command) =>
    `<div class="grid gap-1 md:grid-cols-[16rem_1fr] md:items-center">
      <p class="text-sm text-gray-600 dark:text-gray-300">${text}</p>
      ${commandBlock(command)}
    </div>`;

  return [
    tip("See this pipeline from your terminal:", `gofer pipeline get ${pipeline.pipeline_id}${namespaceFlag()}`),
    tip("Start a run:", `gofer pipeline run ${pipeline.pipeline_id}${namespaceFlag()}`),
  ].join("");
}

function renderRunTable(runs) {
  if (runs.length === 0) {
    return `<tr><td colspan="6" class="px-4 py-8 text-center text-gray-500 dark:text-gray-400">No runs yet.</td></tr>`;
  }

  return runs
    .map((run) => {
      const link = escapeHtml(runDetailsLink(run));
      const finished = lower(run.state) === "complete";
      const reason = run.status_reason ? `${run.status_reason.reason}: ${run.status_reason.description}` : "";
      const duration = run.started ? compactDuration((run.ended || Date.now()) - run.started) : "-";

      return `<tr data-href="${link}" class="cursor-pointer hover:bg-gray-50 dark:hover:bg-neutral-700/40">
          <td class="px-4 py-2.5 whitespace-nowrap"><a href="${link}" class="font-medium hover:text-emerald-600">#${escapeHtml(run.run_id)}</a></td>
          <td class="px-4 py-2.5" ${reason ? `title="${escapeHtml(reason)}"` : ""}>${statusBadge(finished ? run.status : run.state)}</td>
          <td class="px-4 py-2.5 whitespace-nowrap" title="${formatTimestamp(run.created)}">${relativeTime(run.created)}</td>
          <td class="px-4 py-2.5 whitespace-nowrap tabular-nums">${duration}</td>
          <td class="px-4 py-2.5 whitespace-nowrap" title="Token ID: ${escapeHtml(run.initiator.id)}">${escapeHtml(run.initiator.user)}</td>
          <td class="px-4 py-2.5 text-right text-gray-400">›</td>
        </tr>`;
    })
    .join("");
}

// Same order as `gofer pipeline get`: tasks with fewer dependencies first, then by id.
function renderTasks(config) {
  const tasks = Object.values(config.tasks).sort(
    (a, b) => Object.keys(a.depends_on).length - Object.keys(b.depends_on).length || a.id.localeCompare(b.id)
  );

  const rows = tasks
    .map(
      (task) => `<tr>
        <td class="px-4 py-2.5 align-top whitespace-nowrap">${taskName(task.id)}
          ${task.description ? `<p class="mt-0.5 text-xs text-gray-500 dark:text-gray-400 whitespace-normal">${escapeHtml(task.description)}</p>` : ""}</td>
        <td class="px-4 py-2.5 align-top font-mono text-xs break-all">${escapeHtml(task.image)}</td>
        <td class="px-4 py-2.5 align-top text-xs">${startsAfter(task.depends_on)}</td>
      </tr>`
    )
    .join("");

  return `<div class="overflow-x-auto rounded-md ring-1 ring-gray-200 dark:ring-neutral-700">
      <table class="min-w-full divide-y divide-gray-200 dark:divide-neutral-700 text-sm">
        <thead class="bg-gray-50 dark:bg-neutral-900/40">
          <tr class="text-left text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">
            <th scope="col" class="px-4 py-3 font-medium">Task</th>
            <th scope="col" class="px-4 py-3 font-medium">Image</th>
            <th scope="col" class="px-4 py-3 font-medium">Starts</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-200 dark:divide-neutral-700">${rows}</tbody>
      </table>
    </div>`;
}

function renderSubscriptions(subscriptions) {
  if (subscriptions.length === 0) {
    return emptyNotice(
      "This pipeline isn't subscribed to any extensions. Subscribe it to one to, for example, run it on a schedule:",
      `gofer pipeline subscribe ${target.pipeline} <extension_id> <label>${namespaceFlag()}`
    );
  }

  const sorted = [...subscriptions].sort(
    (a, b) => a.subscription_id.localeCompare(b.subscription_id) || a.extension_id.localeCompare(b.extension_id)
  );

  const rows = sorted
    .map(
      (subscription) => `<tr>
        <td class="px-4 py-2.5 whitespace-nowrap font-mono text-xs">${escapeHtml(subscription.subscription_id)}</td>
        <td class="px-4 py-2.5 whitespace-nowrap font-mono text-xs">${escapeHtml(subscription.extension_id)}</td>
        <td class="px-4 py-2.5">${statusBadge(subscription.status)}</td>
        <td class="px-4 py-2.5 text-xs text-gray-600 dark:text-gray-300">${escapeHtml(subscription.status_reason?.description || "")}</td>
      </tr>`
    )
    .join("");

  return `<div class="overflow-x-auto rounded-md ring-1 ring-gray-200 dark:ring-neutral-700">
      <table class="min-w-full divide-y divide-gray-200 dark:divide-neutral-700 text-sm">
        <thead class="bg-gray-50 dark:bg-neutral-900/40">
          <tr class="text-left text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">
            <th scope="col" class="px-4 py-3 font-medium">Subscription</th>
            <th scope="col" class="px-4 py-3 font-medium">Extension</th>
            <th scope="col" class="px-4 py-3 font-medium">Status</th>
            <th scope="col" class="px-4 py-3 font-medium">Reason</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-200 dark:divide-neutral-700">${rows}</tbody>
      </table>
    </div>`;
}

// ---------------------------------------------------------------------------------------------------------------
// Interaction
// ---------------------------------------------------------------------------------------------------------------

document.addEventListener("click", (event) => {
  const row = event.target.closest("tr[data-href]");
  if (row && !event.target.closest("a, button")) {
    window.location.href = row.dataset.href;
  }
});

document.addEventListener("DOMContentLoaded", async function () {
  // A new token can unlock the tasks and subscriptions, and polling may have stopped, so reload and restart it.
  await setupHeader({
    onSessionChange: () => {
      loadPipeline();
      startPolling();
    },
  });

  startPolling();
  await loadPipeline();
});
