// Helpers shared by every page: API access, token handling, theming, and formatting.

// Calls the Gofer API and returns the status alongside the body so callers can tell "not found" apart from
// "not allowed" without digging through exceptions.
async function apiFetch(path) {
  const headers = {
    "Content-Type": "application/json",
    "gofer-api-version": "v0",
  };

  const token = getApiToken();
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }

  try {
    const response = await fetch(path, { method: "GET", headers });
    let data = null;
    try {
      data = await response.json();
    } catch (_) {
      // Some errors come back without a JSON body; the status is enough to go on.
    }
    return { ok: response.ok, status: response.status, data };
  } catch (error) {
    console.error(`Error fetching ${path}:`, error);
    return { ok: false, status: 0, data: null };
  }
}

// Anything that came from the API goes through here before it touches innerHTML. Usernames, status reasons, and
// variable values are all user controlled.
function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

// ---------------------------------------------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------------------------------------------

function saveApiKey(apiKey) {
  document.cookie = `apiToken=${encodeURIComponent(apiKey)}; max-age=${30 * 24 * 60 * 60}; path=/; secure; samesite=strict`;
}

function clearApiKey() {
  document.cookie = "apiToken=; max-age=0; path=/; secure; samesite=strict";
}

function isTokenSaved() {
  return getApiToken() !== "";
}

function getApiToken() {
  const cookieName = "apiToken";

  for (const cookie of document.cookie.split(";")) {
    const trimmed = cookie.trim();
    if (trimmed.startsWith(`${cookieName}=`)) {
      return decodeURIComponent(trimmed.substring(cookieName.length + 1));
    }
  }
  return "";
}

// Who the saved token belongs to, or null when there's no token or it couldn't be verified. Pages read this after
// setupHeader resolves.
let session = null;

// "anonymous", "verified", or "unverified". Unverified means we have a token but whoami didn't answer for it, which
// is normal when the server runs with auth bypassed in development.
let sessionState = "anonymous";

async function loadSession() {
  if (!isTokenSaved()) {
    session = null;
    sessionState = "anonymous";
    return;
  }

  const resp = await apiFetch("/api/tokens/whoami");
  session = resp.ok ? resp.data.token : null;
  sessionState = resp.ok ? "verified" : "unverified";
}

// ---------------------------------------------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------------------------------------------

const ICONS = {
  sun: `<svg viewBox="0 0 20 20" fill="currentColor" class="size-5" aria-hidden="true"><path d="M10 2a.75.75 0 0 1 .75.75v1.5a.75.75 0 0 1-1.5 0v-1.5A.75.75 0 0 1 10 2Zm0 13a.75.75 0 0 1 .75.75v1.5a.75.75 0 0 1-1.5 0v-1.5A.75.75 0 0 1 10 15Zm0-9a4 4 0 1 0 0 8 4 4 0 0 0 0-8Zm5.657-1.596a.75.75 0 0 1 0 1.06l-1.06 1.061a.75.75 0 1 1-1.061-1.06l1.06-1.061a.75.75 0 0 1 1.061 0Zm-9.193 9.192a.75.75 0 0 1 0 1.06l-1.06 1.061a.75.75 0 0 1-1.061-1.06l1.06-1.061a.75.75 0 0 1 1.061 0ZM18 10a.75.75 0 0 1-.75.75h-1.5a.75.75 0 0 1 0-1.5h1.5A.75.75 0 0 1 18 10ZM5 10a.75.75 0 0 1-.75.75h-1.5a.75.75 0 0 1 0-1.5h1.5A.75.75 0 0 1 5 10Zm9.596 5.657a.75.75 0 0 1-1.06 0l-1.061-1.06a.75.75 0 1 1 1.06-1.061l1.061 1.06a.75.75 0 0 1 0 1.061ZM6.404 6.464a.75.75 0 0 1-1.06 0l-1.061-1.06a.75.75 0 0 1 1.06-1.061l1.061 1.06a.75.75 0 0 1 0 1.061Z"/></svg>`,
  moon: `<svg viewBox="0 0 20 20" fill="currentColor" class="size-5" aria-hidden="true"><path fill-rule="evenodd" d="M7.455 2.004a.75.75 0 0 1 .26.77 7 7 0 0 0 9.958 7.967.75.75 0 0 1 1.067.853A8.5 8.5 0 1 1 6.647 1.921a.75.75 0 0 1 .808.083Z" clip-rule="evenodd"/></svg>`,
  key: `<svg viewBox="0 0 20 20" fill="currentColor" class="size-4" aria-hidden="true"><path fill-rule="evenodd" d="M8 7a5 5 0 1 1 3.61 4.804l-1.903 1.903A1 1 0 0 1 9 14H8v1a1 1 0 0 1-1 1H6v1a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1v-2a1 1 0 0 1 .293-.707L8.196 8.39A5.002 5.002 0 0 1 8 7Zm5-3a.75.75 0 0 0 0 1.5A1.5 1.5 0 0 1 14.5 7 .75.75 0 0 0 16 7a3 3 0 0 0-3-3Z" clip-rule="evenodd"/></svg>`,
  external: `<svg viewBox="0 0 20 20" fill="currentColor" class="size-3.5" aria-hidden="true"><path fill-rule="evenodd" d="M4.25 5.5a.75.75 0 0 0-.75.75v8.5c0 .414.336.75.75.75h8.5a.75.75 0 0 0 .75-.75v-4a.75.75 0 0 1 1.5 0v4A2.25 2.25 0 0 1 12.75 17h-8.5A2.25 2.25 0 0 1 2 14.75v-8.5A2.25 2.25 0 0 1 4.25 4h5a.75.75 0 0 1 0 1.5h-5Zm7.25-.75a.75.75 0 0 1 .75-.75h3.5a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0V6.56l-5.22 5.22a.75.75 0 1 1-1.06-1.06l5.22-5.22h-1.69a.75.75 0 0 1-.75-.75Z" clip-rule="evenodd"/></svg>`,
};

function navLink(href, label, active, external = false) {
  const base = "inline-flex items-center gap-1 rounded-md px-3 py-1.5 text-sm";
  const state = active
    ? "bg-gray-100 dark:bg-neutral-700 font-medium text-gray-900 dark:text-gray-100"
    : "text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-gray-50 dark:hover:bg-neutral-700/50";
  const attrs = external ? ` target="_blank" rel="noopener"` : "";
  return `<a href="${href}" class="${base} ${state}"${attrs}>${label}${external ? ICONS.external : ""}</a>`;
}

// The menu behind the account button: a sign-in form when anonymous, token details and sign out otherwise.
function accountPanel() {
  if (sessionState === "anonymous") {
    return `<form id="token-form" class="space-y-3" autocomplete="off">
        <div>
          <label for="token-input" class="block text-sm font-medium">API token</label>
          <p class="mt-0.5 text-xs text-gray-500 dark:text-gray-400">Unlocks other namespaces and the runs you started. It's kept in a cookie on this browser.</p>
        </div>
        <input id="token-input" type="password" autocomplete="off" spellcheck="false" placeholder="Paste your token"
          class="block w-full rounded-md border-0 bg-white dark:bg-neutral-900 px-3 py-2 text-sm font-mono text-gray-900 dark:text-gray-100 ring-1 ring-inset ring-gray-300 dark:ring-neutral-600 placeholder:font-sans placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-emerald-500">
        <p id="token-error" class="hidden text-xs text-red-600 dark:text-red-400"></p>
        <button type="submit" class="w-full rounded-md bg-emerald-600 px-3 py-2 text-sm font-medium text-white hover:bg-emerald-500 disabled:opacity-50">Sign in</button>
        <div class="space-y-1 border-t border-gray-200 dark:border-neutral-700 pt-3">
          <p class="text-xs text-gray-500 dark:text-gray-400">Don't have one? An admin can make you one:</p>
          ${commandBlock("gofer token create <username>")}
        </div>
      </form>`;
  }

  const details =
    sessionState === "verified"
      ? `<p class="text-sm">Signed in as <span class="font-medium">${escapeHtml(session.user)}</span></p>
        <dl class="mt-2 space-y-1 text-xs text-gray-500 dark:text-gray-400">
          <div class="flex justify-between gap-4"><dt>Token</dt><dd class="font-mono truncate">${escapeHtml(session.id)}</dd></div>
          <div class="flex justify-between gap-4"><dt>Expires</dt><dd>${session.expires ? formatTimestamp(session.expires) : "Never"}</dd></div>
          ${session.roles.length ? `<div class="flex justify-between gap-4"><dt>Roles</dt><dd class="text-right">${session.roles.map(escapeHtml).join(", ")}</dd></div>` : ""}
        </dl>`
      : `<p class="text-sm">A token is saved, but the server didn't say who it belongs to.</p>
        <p class="mt-1 text-xs text-gray-500 dark:text-gray-400">That's expected when auth is turned off in development.</p>`;

  return `<div class="space-y-3">
      <div>${details}</div>
      ${commandBlock("gofer token whoami")}
      <button type="button" data-sign-out class="w-full rounded-md px-3 py-2 text-sm text-red-600 dark:text-red-400 ring-1 ring-inset ring-gray-200 dark:ring-neutral-700 hover:bg-red-50 dark:hover:bg-red-950/40">Sign out</button>
    </div>`;
}

function accountButton() {
  if (sessionState === "anonymous") {
    return `${ICONS.key}<span>Sign in</span>`;
  }

  const name = sessionState === "verified" ? session.user : "Token saved";
  return `<span class="grid size-6 place-items-center rounded-full bg-emerald-600 text-xs font-semibold text-white">${escapeHtml(name.charAt(0).toUpperCase())}</span>
    <span class="max-w-[10rem] truncate">${escapeHtml(name)}</span>`;
}

function applyTheme(theme) {
  const dark = theme === "dark";
  document.documentElement.classList.toggle("dark", dark);
  try {
    localStorage.setItem("theme", theme);
  } catch (_) {
    // Private windows can refuse storage; the theme just won't stick.
  }
  const button = document.getElementById("theme-toggle");
  if (button) {
    button.innerHTML = dark ? ICONS.sun : ICONS.moon;
    button.setAttribute("aria-label", dark ? "Switch to light theme" : "Switch to dark theme");
  }
}

function renderAccount() {
  document.getElementById("account-button").innerHTML = accountButton();
  document.getElementById("account-panel").innerHTML = accountPanel();
}

function setAccountOpen(open) {
  document.getElementById("account-panel").classList.toggle("hidden", !open);
  document.getElementById("account-button").setAttribute("aria-expanded", String(open));
  if (open) {
    document.getElementById("token-input")?.focus();
  }
}

// Draws the shared top bar into <header id="site-header">, wires up the theme and account controls, and resolves
// once we know who's signed in. `onSessionChange` runs after a sign in or sign out so the page can reload data.
async function setupHeader({ active = "", onSessionChange = () => {} } = {}) {
  const header = document.getElementById("site-header");
  header.className = "sticky top-0 z-30 border-b border-gray-200 dark:border-neutral-700 bg-white/90 dark:bg-neutral-800/90 backdrop-blur";
  header.innerHTML = `<div class="mx-auto flex h-16 max-w-6xl items-center gap-6 px-6">
      <a href="/" class="flex items-baseline gap-2">
        <span class="text-2xl font-semibold tracking-tight text-transparent bg-clip-text bg-gradient-to-r from-emerald-400 to-emerald-700 dark:to-emerald-200">Gofer</span>
        <span id="version" class="rounded-full bg-gray-100 dark:bg-neutral-700 px-2 py-0.5 text-[11px] font-mono text-gray-500 dark:text-gray-400"></span>
      </a>
      <span class="hidden lg:block text-sm text-gray-500 dark:text-gray-400">Run short lived jobs easily.</span>
      <nav class="ml-auto hidden md:flex items-center gap-1">
        ${navLink("/", "Runs", active === "runs")}
        ${navLink("/docs", "Docs", false)}
        ${navLink("/docs/api_reference.html", "API", false)}
        ${navLink("https://github.com/clintjedwards/gofer", "GitHub", false, true)}
      </nav>
      <div class="ml-auto md:ml-2 flex items-center gap-2">
        <button id="theme-toggle" type="button" class="grid size-9 place-items-center rounded-md text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-neutral-700 hover:text-gray-900 dark:hover:text-gray-100"></button>
        <div class="relative">
          <button id="account-button" type="button" aria-haspopup="true" aria-expanded="false"
            class="flex items-center gap-2 rounded-md px-3 py-1.5 text-sm ring-1 ring-inset ring-gray-200 dark:ring-neutral-700 hover:bg-gray-50 dark:hover:bg-neutral-700/50"></button>
          <div id="account-panel" class="hidden absolute right-0 mt-2 w-80 rounded-lg bg-white dark:bg-neutral-800 p-4 shadow-xl ring-1 ring-gray-200 dark:ring-neutral-700"></div>
        </div>
      </div>
    </div>`;

  applyTheme(document.documentElement.classList.contains("dark") ? "dark" : "light");
  document.getElementById("theme-toggle").addEventListener("click", () => {
    applyTheme(document.documentElement.classList.contains("dark") ? "light" : "dark");
  });

  document.getElementById("account-button").addEventListener("click", () => {
    setAccountOpen(document.getElementById("account-panel").classList.contains("hidden"));
  });

  // Clicking anywhere outside the menu, or pressing Escape, closes it.
  document.addEventListener("click", (event) => {
    if (!event.target.closest("#account-panel, #account-button")) setAccountOpen(false);
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") setAccountOpen(false);
  });

  document.getElementById("account-panel").addEventListener("submit", async (event) => {
    event.preventDefault();
    const input = document.getElementById("token-input");
    const error = document.getElementById("token-error");
    const button = event.target.querySelector("button[type=submit]");
    const token = input.value.trim();
    if (!token) return;

    // Check the token before keeping it. Only an auth failure means it's wrong; a server with auth turned off
    // can't vouch for any token, so anything else is accepted.
    button.disabled = true;
    const resp = await fetch("/api/tokens/whoami", {
      headers: { "gofer-api-version": "v0", Authorization: `Bearer ${token}` },
    }).catch(() => null);
    button.disabled = false;

    if (resp && (resp.status === 401 || resp.status === 403)) {
      error.textContent = "That token wasn't accepted. It may be mistyped, expired, or disabled.";
      error.classList.remove("hidden");
      input.select();
      return;
    }

    saveApiKey(token);
    await loadSession();
    renderAccount();
    setAccountOpen(false);
    onSessionChange();
  });

  document.getElementById("account-panel").addEventListener("click", async (event) => {
    if (!event.target.closest("[data-sign-out]")) return;
    clearApiKey();
    await loadSession();
    renderAccount();
    setAccountOpen(false);
    onSessionChange();
  });

  apiFetch("/api/system/metadata").then((resp) => {
    if (resp.ok && resp.data) {
      document.getElementById("version").textContent = "v" + resp.data.semver;
    }
  });

  await loadSession();
  renderAccount();
}

function formatTimestamp(timestamp) {
  if (timestamp == 0) {
    return "-";
  }

  const date = new Date(timestamp);

  const year = date.getFullYear();
  const month = date.toLocaleString("en-US", { month: "short" });
  const day = date.getDate();
  const hours = date.getHours().toString().padStart(2, "0");
  const minutes = date.getMinutes().toString().padStart(2, "0");
  const seconds = date.getSeconds().toString().padStart(2, "0");
  const timeZone = date.toLocaleString("en-US", { timeZoneName: "short" }).split(" ").pop();

  const daySuffix = (day) => {
    if (day > 3 && day < 21) return "th"; // 11th, 12th, 13th
    switch (day % 10) {
      case 1:
        return "st";
      case 2:
        return "nd";
      case 3:
        return "rd";
      default:
        return "th";
    }
  };

  return `${month} ${day}${daySuffix(day)}, ${year} ${hours}:${minutes}:${seconds} ${timeZone}`;
}

function humanizeDuration(startTimestamp, endTimestamp) {
  const durationMs = endTimestamp - startTimestamp;

  const msPerSecond = 1000;
  const msPerMinute = msPerSecond * 60;
  const msPerHour = msPerMinute * 60;
  const msPerDay = msPerHour * 24;

  const days = Math.floor(durationMs / msPerDay);
  const hours = Math.floor((durationMs % msPerDay) / msPerHour);
  const minutes = Math.floor((durationMs % msPerHour) / msPerMinute);
  const seconds = Math.floor((durationMs % msPerMinute) / msPerSecond);
  const milliseconds = durationMs % msPerSecond;

  const parts = [];

  if (days > 0) parts.push(`${days} day${days !== 1 ? "s" : ""}`);
  if (hours > 0) parts.push(`${hours} hour${hours !== 1 ? "s" : ""}`);
  if (minutes > 0) parts.push(`${minutes} minute${minutes !== 1 ? "s" : ""}`);
  if (seconds > 0) parts.push(`${seconds} second${seconds !== 1 ? "s" : ""}`);
  if (milliseconds > 0 && parts.length === 0) {
    // Only bother with milliseconds when there's no larger unit to show.
    parts.push(`${milliseconds} ms`);
  }

  return parts.join(", ") || "0 ms";
}

function generateStatusColor(status) {
  switch (String(status).toLowerCase()) {
    case "pending":
    case "processing":
    case "waiting":
    case "running":
      return "dark:ring-yellow-50/10 ring-yellow-600/20 dark:bg-yellow-800 bg-yellow-50 dark:text-yellow-50 text-yellow-700";
    case "complete":
    case "successful":
      return "dark:ring-emerald-50/10 ring-emerald-600/20 dark:bg-emerald-800 bg-emerald-50 dark:text-emerald-50 text-emerald-700";
    case "failed":
      return "dark:ring-red-50/10 ring-red-600/20 dark:bg-red-800 bg-red-50 dark:text-red-50 text-red-700";
    case "cancelled":
    case "skipped":
      return "dark:ring-slate-50/10 ring-slate-600/20 dark:bg-slate-800 bg-slate-50 dark:text-slate-50 text-slate-700";
    default:
      return "dark:ring-purple-50/10 ring-purple-600/20 dark:bg-purple-800 bg-purple-50 dark:text-purple-50 text-purple-700";
  }
}

// "just now", "4m ago", "3h ago", "2d ago". Exact times go in a title attribute next to it.
function relativeTime(timestamp) {
  if (!timestamp) {
    return "never";
  }
  const secs = Math.floor((Date.now() - timestamp) / 1000);
  if (secs < 45) return "just now";
  if (secs < 3600) return `${Math.max(1, Math.round(secs / 60))}m ago`;
  if (secs < 86400) return `${Math.round(secs / 3600)}h ago`;
  return `${Math.round(secs / 86400)}d ago`;
}

// Only touches the DOM when something actually changed, so scroll positions, hover states, and text selections
// survive the refresh loop.
function setHtml(id, html) {
  const el = document.getElementById(id);
  if (el.dataset.rendered !== html) {
    el.innerHTML = html;
    el.dataset.rendered = html;
  }
}

// A CLI command the user can copy. The clipboard API only exists on secure origins, so on plain http the text is
// still click-to-select.
function commandBlock(command) {
  const copy = navigator.clipboard
    ? `<button type="button" data-copy="${escapeHtml(command)}" class="shrink-0 rounded px-2 py-0.5 text-xs text-gray-500 hover:text-emerald-600 hover:bg-gray-200 dark:hover:bg-neutral-700">copy</button>`
    : "";
  return `<div class="flex items-center gap-2 rounded bg-gray-100 dark:bg-neutral-900 px-3 py-1.5">
      <span class="text-gray-400 select-none">$</span>
      <code class="flex-1 select-all font-mono text-xs text-gray-800 dark:text-gray-200">${escapeHtml(command)}</code>
      ${copy}
    </div>`;
}

document.addEventListener("click", (event) => {
  const copy = event.target.closest("[data-copy]");
  if (copy) {
    event.preventDefault();
    navigator.clipboard.writeText(copy.dataset.copy).then(() => {
      copy.textContent = "copied";
      setTimeout(() => (copy.textContent = "copy"), 1500);
    });
  }
});

// A small colored pill for states and statuses, used in the run list and the details page.
function statusBadge(status) {
  return `<span class="${generateStatusColor(status)} inline-block min-w-[12ch] ring-1 ring-inset rounded-sm text-xs text-center px-2 py-1">${escapeHtml(status)}</span>`;
}
