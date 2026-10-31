// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later
// The window: draws the app's state whenever the app says it changed, and sends the
// owner's choices back. Plain JavaScript over Tauri's global API; no build step.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);
let current = null;

// Shows a command's error in the window (a webview may not show alert()), for 8 s.
let errorTimer = null;
function showError(e) {
  const box = $("error");
  box.textContent = String(e);
  box.hidden = false;
  clearTimeout(errorTimer);
  errorTimer = setTimeout(() => { box.hidden = true; }, 8000);
}
let editingWords = false;

function el(tag, attrs = {}, ...children) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === "class") e.className = v;
    else if (k.startsWith("on")) e.addEventListener(k.slice(2), v);
    else e.setAttribute(k, v);
  }
  for (const c of children) e.append(c instanceof Node ? c : document.createTextNode(String(c)));
  return e;
}

function list(target, items, empty, render) {
  const ul = $(target);
  ul.replaceChildren(...(items.length ? items.map(render) : [el("li", { class: "empty" }, empty)]));
}

function time(iso) {
  return new Date(iso).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

function stateLine(s) {
  const v = s.voice;
  const speaking = s.queue.find((i) => i.state === "speaking");
  if (v.state === "downloading") {
    const pct = v.total ? Math.floor((100 * (v.done || 0)) / v.total) : 0;
    return ["busy", `Downloading the voice, first run only: ${pct}%`];
  }
  if (v.state === "starting") return ["busy", "Starting the voice…"];
  if (v.state === "failed") return ["failed", `The voice stopped: ${v.detail || "unknown"}. Restarting…`];
  if (speaking && speaking.speak === false) return ["busy", `Preparing speech to keep for ${speaking.client}`];
  if (speaking) return ["busy", `Speaking for ${speaking.client}`];
  return ["ready", "Ready"];
}

function draw(s) {
  current = s;
  const [cls, line] = stateLine(s);
  $("dot").className = `dot ${cls}`;
  $("state").textContent = line;

  const voice = $("voice");
  if (!voice.options.length) for (const v of s.voices) voice.append(el("option", { value: v }, v));
  if (document.activeElement !== voice) voice.value = s.settings.voice;
  $("by-clause").checked = s.settings.by_clause;
  if (document.activeElement !== $("speed")) $("speed").value = s.settings.speed;
  $("speed-value").textContent = `${Number(s.settings.speed).toFixed(2)}×`;

  list("queue", s.queue, "Nothing to say.", (i) =>
    el("li", { class: i.state },
      el("span", { class: "meta" }, i.state !== "speaking" ? "waiting" : i.speak === false ? "preparing" : "speaking"),
      el("span", { class: "text" }, i.text),
      el("span", { class: "meta" }, i.client),
      el("button", { class: "small", onclick: () => invoke("withdraw", { id: i.id }) }, "Withdraw")));

  list("readers", s.readers, "No clients connected.", (r) =>
    el("li", {}, el("span", { class: "text" }, r.name), el("span", { class: "meta" }, `${r.via}, since ${time(r.since)}`)));

  list("recent", s.recent, "Nothing yet.", (r) =>
    el("li", { class: r.outcome },
      el("span", { class: "meta" }, time(r.at)),
      el("span", { class: "text" }, r.text),
      el("span", { class: "meta" }, r.outcome === "spoken" ? r.client : `${r.client}, ${r.outcome}`)));

  if (!editingWords) drawWords(s.settings.words);
  $("lan").checked = s.settings.lan;
  $("port").textContent = s.port;
  $("lan-error").hidden = !s.lan_error;
  $("lan-error").textContent = s.lan_error || "";
  list("tokens", s.settings.tokens, "No tokens issued.", (t) =>
    el("li", {}, el("span", { class: "text" }, t.name), el("span", { class: "meta" }, `issued ${new Date(t.created).toLocaleDateString()}`),
      el("button", { class: "small", onclick: () => invoke("revoke_token", { id: t.id }).then(refresh) }, "Revoke")));
  $("autostart").checked = s.autostart;
  $("version").textContent = s.version;
  $("socket").textContent = s.socket;
}

function drawWords(words) {
  const rows = Object.entries(words).map(([w, said]) =>
    el("tr", {},
      el("td", {}, w),
      el("td", {}, said),
      el("td", {}, el("button", { class: "small", onclick: () => invoke("try_say", { text: w }) }, "Say it")),
      el("td", {}, el("button", { class: "small", onclick: () => saveWords(Object.fromEntries(Object.entries(words).filter(([k]) => k !== w))) }, "Remove"))));
  $("words").replaceChildren(...rows);
}

async function saveWords(words) {
  try { await invoke("set_words", { words }); } catch (e) { showError(e); }
  refresh();
}

async function refresh() {
  try { draw(await invoke("app_state")); } catch (e) { $("state").textContent = String(e); }
}

$("toggle-settings").onclick = () => {
  const open = $("settings").hidden;
  $("settings").hidden = !open;
  $("main").hidden = open;
  $("toggle-settings").textContent = open ? "Done" : "Settings";
};
$("stop").onclick = () => invoke("stop");
const setDefaults = () => invoke("set_defaults", { voice: $("voice").value, speed: Number($("speed").value), byClause: $("by-clause").checked }).then(refresh).catch(showError);
$("voice").onchange = setDefaults;
$("speed").onchange = setDefaults;
$("by-clause").onchange = setDefaults;
$("speed").oninput = () => { $("speed-value").textContent = `${Number($("speed").value).toFixed(2)}×`; };
const trySay = () => { const t = $("try-text").value; if (t.trim()) invoke("try_say", { text: t }).catch(showError); };
$("try").onclick = trySay;
$("try-text").onkeydown = (e) => { if (e.key === "Enter") trySay(); };
$("add-word").onclick = () => {
  const w = $("new-word").value.trim(), said = $("new-said").value.trim();
  if (!w || !said) return;
  $("new-word").value = ""; $("new-said").value = "";
  saveWords({ ...current.settings.words, [w.toLowerCase()]: said });
};
for (const id of ["new-word", "new-said"]) {
  $(id).onfocus = () => { editingWords = true; };
  $(id).onblur = () => { editingWords = false; };
}
$("lan").onchange = () => invoke("set_lan", { on: $("lan").checked }).catch(showError).then(refresh);
$("issue").onclick = async () => {
  try {
    const secret = await invoke("issue_token", { name: $("token-name").value });
    $("token-name").value = "";
    $("secret").hidden = false;
    $("secret").textContent = `Copy this token now; it is not shown again: ${secret}`;
  } catch (e) { showError(e); }
  refresh();
};
$("autostart").onchange = () => invoke("set_autostart", { on: $("autostart").checked }).catch(showError).then(refresh);

listen("changed", refresh);
refresh();
setInterval(refresh, 5000);
