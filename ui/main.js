const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;

const statusPill = document.getElementById("status-pill");
const listenBtn = document.getElementById("listen-btn");
const updateBtn = document.getElementById("update-btn");
const phrasesEl = document.getElementById("phrases");
const saveBtn = document.getElementById("save-phrases");
const hitsEl = document.getElementById("hits");
const modelNote = document.getElementById("model-note");
const modelProgress = document.getElementById("model-progress");

let listening = false;

function setStatus(text, cls) {
  statusPill.textContent = text;
  statusPill.className = `pill ${cls}`;
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

await listen("hit", (event) => {
  const hit = event.payload;
  const li = document.createElement("li");
  li.textContent = `${new Date().toLocaleTimeString()}  ·  ${hit.phrase}  ·  ${hit.transcript || hit.source}`;
  hitsEl.prepend(li);
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
