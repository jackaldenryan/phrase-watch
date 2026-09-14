const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;

const COLORS = [
  "#f59e0b",
  "#3d9cf0",
  "#34d399",
  "#e06c75",
  "#c678dd",
  "#56b6c2",
  "#e5c07b",
  "#61afef",
];

const statusPill = document.getElementById("status-pill");
const listenBtn = document.getElementById("listen-btn");
const updateBtn = document.getElementById("update-btn");
const phrasesEl = document.getElementById("phrases");
const saveBtn = document.getElementById("save-phrases");
const hitsEl = document.getElementById("hits");
const modelNote = document.getElementById("model-note");
const modelProgress = document.getElementById("model-progress");
const bucketEl = document.getElementById("bucket");
const presetEl = document.getElementById("preset");
const phraseFilter = document.getElementById("phrase-filter");
const customRange = document.getElementById("custom-range");
const fromDate = document.getElementById("from-date");
const toDate = document.getElementById("to-date");
const statsTotal = document.getElementById("stats-total");
const rangeLabel = document.getElementById("range-label");
const chart = document.getElementById("chart");
const legend = document.getElementById("legend");
const breakdown = document.getElementById("breakdown");
const statsBanner = document.getElementById("stats-banner");

let listening = false;

function setStatus(text, cls) {
  statusPill.textContent = text;
  statusPill.className = `pill ${cls}`;
}

function startOfDay(date) {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function startOfWeek(date) {
  const day = startOfDay(date);
  const weekday = day.getDay();
  day.setDate(day.getDate() + (weekday === 0 ? -6 : 1 - weekday));
  return day;
}

function rangeFromPreset(preset) {
  const now = new Date();
  const end = now.getTime();
  const day = 24 * 60 * 60 * 1000;
  switch (preset) {
    case "wtd":
      return { start: startOfWeek(now).getTime(), end, label: "Week to date" };
    case "mtd":
      return { start: new Date(now.getFullYear(), now.getMonth(), 1).getTime(), end, label: "Month to date" };
    case "ytd":
      return { start: new Date(now.getFullYear(), 0, 1).getTime(), end, label: "Year to date" };
    case "24h":
      return { start: end - day, end, label: "24 hours" };
    case "1w":
      return { start: end - 7 * day, end, label: "1 week" };
    case "2w":
      return { start: end - 14 * day, end, label: "2 weeks" };
    case "1mo":
      return { start: end - 30 * day, end, label: "1 month" };
    case "6mo":
      return { start: end - 180 * day, end, label: "6 months" };
    case "1y":
      return { start: end - 365 * day, end, label: "1 year" };
    case "all":
      return { start: null, end, label: "All time" };
    case "custom": {
      const from = fromDate.value ? startOfDay(new Date(`${fromDate.value}T00:00:00`)).getTime() : startOfDay(now).getTime();
      const to = toDate.value ? new Date(`${toDate.value}T23:59:59`).getTime() : end;
      return { start: from, end: Math.min(to, end), label: "Custom" };
    }
    default:
      return { start: startOfDay(now).getTime(), end, label: "Today" };
  }
}

function formatTick(ms, bucket) {
  const d = new Date(ms);
  if (bucket === "1mo") return d.toLocaleDateString(undefined, { month: "short", year: "2-digit" });
  if (bucket === "1w" || bucket === "1d") return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  return d.toLocaleTimeString(undefined, { hour: "numeric", minute: bucket === "15m" ? "2-digit" : undefined });
}

function colorFor(phrase, phrases) {
  const i = Math.max(0, phrases.indexOf(phrase));
  return COLORS[i % COLORS.length];
}

function renderChart(stats, bucket) {
  const phrases = stats.breakdown.map((item) => item.phrase);
  const w = 640;
  const h = 220;
  const pad = { l: 36, r: 8, t: 10, b: 36 };
  const innerW = w - pad.l - pad.r;
  const innerH = h - pad.t - pad.b;
  const max = Math.max(1, ...stats.buckets.map((b) => b.count));
  const n = Math.max(1, stats.buckets.length);
  const gap = n > 40 ? 1 : 3;
  const barW = Math.max(2, innerW / n - gap);
  const tickEvery = n > 16 ? Math.ceil(n / 10) : 1;

  let svg = "";
  for (let g = 1; g <= 4; g++) {
    const y = pad.t + innerH - (innerH * g) / 4;
    svg += `<line x1="${pad.l}" x2="${w - pad.r}" y1="${y}" y2="${y}" class="grid" />`;
    svg += `<text x="${pad.l - 6}" y="${y + 3}" class="axis" text-anchor="end">${Math.round((max * g) / 4)}</text>`;
  }

  stats.buckets.forEach((bucketRow, i) => {
    const x = pad.l + i * (innerW / n) + gap / 2;
    let y = pad.t + innerH;
    const segs = bucketRow.segments.length ? bucketRow.segments : [];
    for (const seg of segs) {
      const bh = (seg.count / max) * innerH;
      y -= bh;
      svg += `<rect x="${x}" y="${y}" width="${barW}" height="${Math.max(bh, 0)}" fill="${colorFor(seg.phrase, phrases)}" rx="1" />`;
    }
    if (i % tickEvery === 0) {
      svg += `<text x="${x + barW / 2}" y="${h - 8}" class="axis" text-anchor="middle">${formatTick(bucketRow.start_ms, bucket)}</text>`;
    }
  });
  chart.innerHTML = svg;

  legend.innerHTML = "";
  for (const item of stats.breakdown) {
    const li = document.createElement("li");
    li.innerHTML = `<span class="swatch" style="background:${colorFor(item.phrase, phrases)}"></span>${item.phrase}`;
    legend.appendChild(li);
  }

  breakdown.innerHTML = "";
  const total = stats.total || 1;
  for (const item of stats.breakdown) {
    const li = document.createElement("li");
    const pct = ((item.count / total) * 100).toFixed(0);
    li.innerHTML = `<span>${item.phrase}</span><span>${item.count}</span><span class="hint">${pct}%</span>`;
    breakdown.appendChild(li);
  }
}

async function refreshStats() {
  const preset = presetEl.value;
  customRange.hidden = preset !== "custom";
  const range = rangeFromPreset(preset);
  rangeLabel.textContent = range.label;
  const stats = await invoke("get_stats", {
    startMs: range.start,
    endMs: range.end,
    phrase: phraseFilter.value || null,
    bucket: bucketEl.value,
    tzOffsetMin: new Date().getTimezoneOffset(),
  });
  statsTotal.textContent = String(stats.total ?? 0);
  const current = phraseFilter.value;
  const options = ["<option value=\"\">All phrases</option>"]
    .concat((stats.phrases || []).map((p) => `<option value="${p.replace(/"/g, "&quot;")}">${p}</option>`));
  phraseFilter.innerHTML = options.join("");
  if (current && (stats.phrases || []).includes(current)) phraseFilter.value = current;
  if (stats.too_many) {
    statsBanner.hidden = false;
    statsBanner.textContent = `That range would make ${stats.too_many} bars. Pick a larger bucket or a shorter range.`;
    chart.innerHTML = "";
    legend.innerHTML = "";
    return;
  }
  statsBanner.hidden = true;
  renderChart(stats, bucketEl.value);
}

function renderHits(hits) {
  hitsEl.innerHTML = "";
  for (const hit of hits.slice().reverse().slice(0, 40)) {
    const li = document.createElement("li");
    const when = new Date(hit.t * 1000).toLocaleTimeString();
    li.textContent = `${when}  ·  ${hit.phrase}  ·  ${hit.transcript || hit.source}`;
    hitsEl.appendChild(li);
  }
}

async function refresh() {
  const state = await invoke("get_state");
  listening = state.listening;
  phrasesEl.value = (state.phrases || []).join("\n");
  renderHits(state.hits || []);
  if (!state.models_ready) {
    setStatus("Needs models", "paused");
    listenBtn.disabled = true;
    modelNote.textContent = "Downloading on-device models…";
    await invoke("ensure_models");
    return refresh();
  }
  modelNote.textContent = "Models are on this Mac. Nothing is sent to the internet while listening.";
  listenBtn.disabled = false;
  listenBtn.textContent = listening ? "Stop listening" : "Start listening";
  setStatus(listening ? "Listening" : "Paused", listening ? "listening" : "paused");
  await refreshStats();
}

listenBtn.addEventListener("click", async () => {
  await invoke("set_listening", { listening: !listening });
  await refresh();
});

saveBtn.addEventListener("click", async () => {
  const phrases = phrasesEl.value
    .split("\n")
    .map((s) => s.trim())
    .filter(Boolean);
  await invoke("set_phrases", { phrases });
  await refresh();
});

updateBtn.addEventListener("click", async () => {
  const result = await invoke("check_updates");
  alert(result);
});

for (const el of [bucketEl, presetEl, phraseFilter, fromDate, toDate]) {
  el.addEventListener("change", () => refreshStats().catch((err) => {
    modelNote.textContent = String(err);
  }));
}

await listen("hit", async (event) => {
  const hit = event.payload;
  const li = document.createElement("li");
  li.textContent = `${new Date().toLocaleTimeString()}  ·  ${hit.phrase}  ·  ${hit.transcript || hit.source}`;
  hitsEl.prepend(li);
  await refreshStats();
});

await listen("model-progress", (event) => {
  modelProgress.hidden = false;
  modelProgress.value = event.payload;
});

await listen("listening-changed", (event) => {
  listening = event.payload;
  listenBtn.textContent = listening ? "Stop listening" : "Start listening";
  setStatus(listening ? "Listening" : "Paused", listening ? "listening" : "paused");
});

await listen("error", (event) => {
  modelNote.textContent = event.payload;
});

try {
  await refresh();
} catch (err) {
  modelNote.textContent = String(err);
}
