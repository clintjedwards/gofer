let displayedRuns = new Set();

async function updateSemver() {
  try {
    const response = await fetch("/api/system/metadata", {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
        "gofer-api-version": "v0",
      },
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const data = await response.json();

    const semver = data.semver;

    document.getElementById("version").textContent = "v" + semver;
  } catch (error) {
    console.error("Error fetching system metadata:", error);
  }
}

async function listAllPipelines(token) {
  try {
    const response = await fetch("/api/namespaces/default/pipelines", {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
        "gofer-api-version": "v0",
        Authorization: `Bearer ${token}`,
      },
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const data = await response.json();
    return data;
  } catch (error) {
    console.error("Error fetching pipeline:", error);
  }
}

async function updateCurrentTime() {
  const now = new Date();
  const utcTime = now.toUTCString();

  document.getElementById("current-time").innerText = utcTime;
}

async function getRunList(token) {
  let runs = [];

  try {
    const resp = await listAllPipelines(token);

    for (const pipeline of resp.pipelines) {
      if (runs.length >= 5) {
        runs.splice(5);
        return runs;
      }

      const response = await fetch(`/api/namespaces/default/pipelines/${pipeline.pipeline_id}/runs?limit=5&reverse=true`, {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          "gofer-api-version": "v0",
          Authorization: `Bearer ${token}`,
        },
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();

      runs = runs.concat(data.runs);
    }
  } catch (error) {
    console.error("Error displaying run list:", error);
  }

  runs.sort((a, b) => b.started - a.started);
  runs.splice(5);
  return runs;
}

function formatTimestampToUTC(timestamp) {
  if (timestamp == 0) {
    return "-";
  }

  // Create a Date object from the timestamp
  const date = new Date(timestamp);

  // Get the components of the date
  const year = date.getUTCFullYear();
  const month = date.toLocaleString("en-US", { month: "short", timeZone: "UTC" });
  const day = date.getUTCDate();
  const hours = date.getUTCHours().toString().padStart(2, "0");
  const minutes = date.getUTCMinutes().toString().padStart(2, "0");
  const seconds = date.getUTCSeconds().toString().padStart(2, "0");

  // Determine the suffix for the day
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

  // Format the date and time
  const formattedDate = `${month} ${day}${daySuffix(day)}, ${year}`;
  const formattedTime = `${hours}:${minutes}:${seconds} UTC`;

  // Combine the formatted date and time
  return `${formattedDate} ${formattedTime}`;
}

function saveApiKey(apiKey) {
  const cookieString = `apiToken=${encodeURIComponent(apiKey)}; max-age=${30 * 24 * 60 * 60}; path=/; secure; samesite=strict`;
  document.cookie = cookieString;
}

function handleEnterPress(event) {
  if (event.key === "Enter") {
    const input = event.target.value.trim(); // Get the trimmed value of the input
    if (isTokenSaved()) {
      return;
    }
    if (input) {
      saveApiKey(input);
      loadDefaultPipelineRuns();
    }
  }
}

function generateNewRunElements(runs) {
  const container = document.getElementById("run-list-body");
  const divs = Array.from(container.children);

  for (const [index, run] of runs.entries()) {
    let div = generateNewRunElement(run);

    if (divs[index]) {
      if (divs[index].innerHTML !== div.innerHTML) {
        divs[index].innerHTML = div.innerHTML;
      }
    } else {
      container.appendChild(div);
    }
  }
}

function humanizeDuration(startTimestamp, endTimestamp) {
  // Calculate the duration in milliseconds
  const durationMs = endTimestamp - startTimestamp;

  // Define time units in milliseconds
  const msPerSecond = 1000;
  const msPerMinute = msPerSecond * 60;
  const msPerHour = msPerMinute * 60;
  const msPerDay = msPerHour * 24;

  // Calculate the time in days, hours, minutes, seconds, and milliseconds
  const days = Math.floor(durationMs / msPerDay);
  const hours = Math.floor((durationMs % msPerDay) / msPerHour);
  const minutes = Math.floor((durationMs % msPerHour) / msPerMinute);
  const seconds = Math.floor((durationMs % msPerMinute) / msPerSecond);
  const milliseconds = durationMs % msPerSecond;

  // Create an array to hold parts of the duration string
  const parts = [];

  // Add each non-zero time component to the parts array
  if (days > 0) parts.push(`${days} day${days !== 1 ? "s" : ""}`);
  if (hours > 0) parts.push(`${hours} hour${hours !== 1 ? "s" : ""}`);
  if (minutes > 0) parts.push(`${minutes} minute${minutes !== 1 ? "s" : ""}`);
  if (seconds > 0) parts.push(`${seconds} second${seconds !== 1 ? "s" : ""}`);
  if (milliseconds > 0 && parts.length === 0) {
    // Include milliseconds if no larger unit is used
    parts.push(`${milliseconds} ms`);
  }

  // Join the parts into a string
  return parts.join(", ") || "0 ms";
}

function generateStatusColor(status) {
    switch (status.toLowerCase()) {
    case "unknown":
      return "ring-purple-600/20 bg-purple-50 text-purple-700";
    case "pending":
    case "running":
      return "ring-yellow-600/20 bg-yellow-50 text-yellow-700";
    case "complete":
    case "successful":
      return "ring-emerald-600/20 bg-emerald-50 text-emerald-700";
    case "failed":
      return "ring-red-600/20 bg-red-50 text-red-700";
    case "cancelled":
      return "ring-slate-600/20 bg-slate-50 text-slate-700";
  }
}


function generateNewRunElement(run) {
  const runElement = document.createElement("tr");
  runElement.className = "overflow-hidden w-[1200px]";

  let duration = 0;

  if (run.ended == 0) {
    const currentTimeInMilliseconds = Date.now();
    duration = humanizeDuration(run.started, currentTimeInMilliseconds);
  } else {
    duration = humanizeDuration(run.started, run.ended);
  }

  const statusReasonTitle = run.status_reason ? `reason: [${run.status_reason.reason}]: ${run.status_reason.description}`: '';
  
  runElement.innerHTML = `
        <td class="text-center whitespace-nowrap py-2 pl-4 pr-3 text-sm text-gray-700 sm:pl-0">${run.namespace_id}</td>
        <td class="text-center whitespace-nowrap px-2 py-2 text-sm font-medium text-gray-900">${run.pipeline_id}</td>
        <td class="text-center whitespace-nowrap px-2 py-2 text-sm text-gray-900">${run.run_id}</td>
        <td title="Token ID: ${run.initiator.id}" class="text-center whitespace-nowrap px-2 py-2 text-sm text-gray-700">${run.initiator.user}</td>
        <td title="duration: ${duration}" class="text-center whitespace-nowrap px-2 py-2 text-sm text-gray-700">${formatTimestampToUTC(run.started)}</td>
        <td title="duration: ${duration}" class="text-center whitespace-nowrap px-2 py-2 text-sm text-gray-700">${formatTimestampToUTC(run.ended)}</td>
        <td ${statusReasonTitle ? `title="${statusReasonTitle}"` : ''} class="text-center whitespace-nowrap"><span class="${generateStatusColor(run.status)} inline-block w-[12ch] ring-1 ring-inset rounded-sm text-xs text-center px-2 py-1">${run.status}</span></td>
        <td class="text-center whitespace-nowrap"><span class="${generateStatusColor(run.state)} inline-block w-[12ch] ring-1 ring-inset rounded-sm text-xs text-center px-2 py-1">${run.state}</span></td>
        <td class="relative whitespace-nowrap py-2 pl-3 pr-4 text-right text-sm font-medium sm:pr-0">
          <a href="#">Details<span class="sr-only">, ${run.pipeline_id}, ${run.run_id}</span></a>
        </td>
    `;
  return runElement;
}

function isTokenSaved() {
  const cookies = document.cookie.split(";");

  // Iterate over the cookies and check if the desired cookie exists
  for (let i = 0; i < cookies.length; i++) {
    const cookie = cookies[i].trim();

    // Check if the cookie name matches the desired cookie
    if (cookie.startsWith(`apiToken`)) {
      return true;
    }
  }
  return false;
}

function getApiToken() {
  const cookies = document.cookie.split(";");
  const cookieName = "apiToken";

  // Iterate over the cookies and check if the desired cookie exists
  for (let i = 0; i < cookies.length; i++) {
    const cookie = cookies[i].trim();

    // Check if the cookie name matches the desired cookie
    if (cookie.startsWith(`${cookieName}=`)) {
      return cookie.substring(cookieName.length + 1);
    }
  }
  return "";
}

async function loadDefaultPipelineRuns() {
  if (!isTokenSaved()) {
    document.getElementById("token-prompt-label").classList.remove("hidden");
    document.getElementById("token-prompt").classList.remove("hidden");
    document.getElementById("token-saved").classList.add("hidden");
    
    let runs = await getRunList("");
    generateNewRunElements(runs);
    
    return;
  }

  document.getElementById("token-prompt-label").classList.add("hidden");
  document.getElementById("token-prompt").classList.add("hidden");
  document.getElementById("token-saved").classList.remove("hidden");

  const token = getApiToken();

  let runs = await getRunList(token);
  generateNewRunElements(runs);
}

document.addEventListener("DOMContentLoaded", async function () {
  updateSemver();
  updateCurrentTime();
  
  await loadDefaultPipelineRuns();

  setInterval(loadDefaultPipelineRuns, 5000); // every 5 seconds
  setInterval(updateCurrentTime, 5000);
});

document.getElementById("token-prompt").addEventListener("keydown", handleEnterPress);
