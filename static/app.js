const USER_ID =
  localStorage.getItem("filmrank_uid") ||
  (() => {
    const id = crypto.randomUUID();
    localStorage.setItem("filmrank_uid", id);
    return id;
  })();

let allFilms = [];
let selectedIds = new Set(
  JSON.parse(localStorage.getItem("filmrank_selected") || "[]"),
);
let currentPair = null;
let voteCount = 0;
let voteHistory = []; // [{winnerId, loserId}, ...]
let focusFilmId = null; // pinned film for compare mode

// -- Helpers --
function esc(s) {
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}

const brokenPosters = new Set();
function posterImg(url) {
  if (!url || brokenPosters.has(url))
    return `<div class="poster-ph">&#127902;</div>`;
  return `<img class="poster" src="${esc(url)}" onerror="brokenPosters.add(this.src);this.outerHTML='<div class=\\'poster-ph\\'>&#127902;</div>'">`;
}

function filmMeta(f) {
  return `${esc(f.team)}${f.city ? " &middot; " + esc(f.city) : ""}`;
}

// -- Init --
async function init() {
  const [filmsRes, boardRes] = await Promise.all([
    fetch("/api/films"),
    fetch("/api/leaderboard"),
  ]);
  allFilms = await filmsRes.json();
  // Preload poster URLs so broken ones are detected before cards render
  allFilms.forEach((f) => {
    if (!f.poster_url) return;
    const img = new Image();
    img.onerror = () => brokenPosters.add(f.poster_url);
    img.src = f.poster_url;
  });
  const board = await boardRes.json();
  const rankById = new Map(board.map((item, i) => [item.film_id, i]));
  allFilms.sort(
    (a, b) =>
      (rankById.get(a.id) ?? Infinity) - (rankById.get(b.id) ?? Infinity),
  );
  renderFilmList(allFilms);
  if (selectedIds.size === 0) {
    allFilms.forEach((f) => selectedIds.add(f.id));
    document
      .querySelectorAll(".film-item")
      .forEach((el) => el.classList.add("selected"));
  }
  updateSelectionStatus();
  if (selectedIds.size >= 2) await saveSelection();
}

// -- Page Navigation --
function showPage(page) {
  document
    .querySelectorAll(".page")
    .forEach((p) => p.classList.remove("active"));
  document
    .querySelectorAll(".nav button")
    .forEach((b) => b.classList.remove("active"));
  document.getElementById(`page-${page}`).classList.add("active");
  document.getElementById(`nav-${page}`).classList.add("active");
  location.hash = page;

  const loaders = { swipe: loadPair, board: loadLeaderboard, more: loadMore };
  if (loaders[page]) loaders[page]();
}

// -- PAGE 1: Film Selection --
function renderFilmList(films) {
  document.getElementById("film-list").innerHTML = films
    .map(
      (f) => `
    <div class="film-item ${selectedIds.has(f.id) ? "selected" : ""}"
         data-id="${f.id}" onclick="toggleFilm(${f.id}, this)">
      <div class="film-check"></div>
      ${posterImg(f.poster_url)}
      <div class="film-info">
        <div class="film-title">${esc(f.title)}</div>
        <div class="film-meta">${filmMeta(f)}</div>
      </div>
    </div>
  `,
    )
    .join("");
}

let saveTimeout = null;

function toggleFilm(id, el) {
  if (selectedIds.has(id)) {
    selectedIds.delete(id);
    el.classList.remove("selected");
  } else {
    selectedIds.add(id);
    el.classList.add("selected");
  }
  updateSelectionStatus();
  debounceSaveSelection();
}

function filterFilms(q) {
  q = q.toLowerCase();
  renderFilmList(
    allFilms.filter(
      (f) =>
        f.title.toLowerCase().includes(q) ||
        f.team.toLowerCase().includes(q) ||
        f.city.toLowerCase().includes(q),
    ),
  );
}

function selectAll() {
  document.querySelectorAll(".film-item").forEach((el) => {
    selectedIds.add(parseInt(el.dataset.id));
    el.classList.add("selected");
  });
  updateSelectionStatus();
  debounceSaveSelection();
}

function deselectAll() {
  selectedIds.clear();
  document
    .querySelectorAll(".film-item")
    .forEach((el) => el.classList.remove("selected"));
  updateSelectionStatus();
  debounceSaveSelection();
}

function updateSelectionStatus() {
  const el = document.getElementById("selection-status");
  const n = selectedIds.size;
  el.textContent =
    n < 2 ? "Select at least 2 films to compare" : `${n} films selected`;
  el.classList.toggle("has-enough", n >= 2);
}

function debounceSaveSelection() {
  clearTimeout(saveTimeout);
  saveTimeout = setTimeout(saveSelection, 500);
}

async function saveSelection() {
  localStorage.setItem("filmrank_selected", JSON.stringify([...selectedIds]));
  await fetch("/api/selection", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ user_id: USER_ID, film_ids: [...selectedIds] }),
  });
}

// -- PAGE 2: Swipe Comparison --
function renderFocusDropdown() {
  const selected = allFilms.filter((f) => selectedIds.has(f.id));
  const options = selected
    .map(
      (f) =>
        `<option value="${f.id}" ${f.id === focusFilmId ? "selected" : ""}>${esc(f.title)}</option>`,
    )
    .join("");
  return `<div class="focus-picker">
    <label for="focus-film">Compare against:</label>
    <select id="focus-film" onchange="setFocusFilm(this.value)">
      <option value="">Random pairs</option>
      ${options}
    </select>
  </div>`;
}

function setFocusFilm(val) {
  focusFilmId = val ? parseInt(val) : null;
  loadPair();
}

async function loadPair() {
  const container = document.getElementById("swipe-container");
  let url = `/api/pair?user_id=${USER_ID}`;
  if (focusFilmId) url += `&focus_film=${focusFilmId}`;
  const res = await fetch(url);
  const data = await res.json();

  voteCount = data.votes || 0;

  if (data.done) {
    const notEnough = selectedIds.size < 2;
    const focusDone = focusFilmId && !notEnough;
    container.innerHTML = `
      ${renderFocusDropdown()}
      <div class="swipe-done">
        <h2>${notEnough ? "Select films first" : "All done!"}</h2>
        <p>${
          notEnough
            ? "Go back and pick at least 2 films you've watched."
            : focusDone
              ? "You've compared this film against all others."
              : `You've compared all possible pairs. (${voteCount} votes cast)`
        }</p>
        <button onclick="${focusDone ? "setFocusFilm('')" : `showPage('${notEnough ? "select" : "board"}')`}">${notEnough ? "Select Films" : focusDone ? "Compare All" : "View Leaderboard"}</button>
      </div>`;
    return;
  }

  currentPair = data;
  renderPair(data.a, data.b);
}

function renderPair(a, b) {
  currentPair = { a, b };
  document.getElementById("swipe-container").innerHTML = `
    <div>
      ${renderFocusDropdown()}
      <div class="vs-badge">VS</div>
      <div class="swipe-arena" id="swipe-arena">
        <div class="film-card" id="film-a">
          ${posterImg(a.poster_url)}
          <div class="title">${esc(a.title)}</div>
          <div class="meta">${filmMeta(a)}</div>
          <button class="deselect-btn" onclick="deselectAndSkip(${a.id}, event)">Haven't seen it</button>
        </div>
        <div class="film-card" id="film-b">
          ${posterImg(b.poster_url)}
          <div class="title">${esc(b.title)}</div>
          <div class="meta">${filmMeta(b)}</div>
          <button class="deselect-btn" onclick="deselectAndSkip(${b.id}, event)">Haven't seen it</button>
        </div>
      </div>
      <div class="swipe-buttons">
        <button class="swipe-arrow-btn pick-a" id="btn-a" title="Pick left">&larr;</button>
        <button class="swipe-arrow-btn pick-b" id="btn-b" title="Pick right">&rarr;</button>
      </div>
      <div class="swipe-progress">${voteCount} comparisons made</div>
      <div class="swipe-bottom-actions">
        <button class="undo-btn" onclick="undoVote()" ${voteHistory.length > 0 ? "" : "disabled"}>Undo</button>
        <button class="skip-btn" onclick="loadPair()">Skip</button>
      </div>
    </div>`;
  setupSwipe();
}

let swipeCleanup = null;

function setupSwipe() {
  if (swipeCleanup) {
    swipeCleanup();
    swipeCleanup = null;
  }

  const arena = document.getElementById("swipe-arena");
  const filmA = document.getElementById("film-a");
  const filmB = document.getElementById("film-b");
  const btnA = document.getElementById("btn-a");
  const btnB = document.getElementById("btn-b");
  if (!arena) return;

  let startX = 0,
    currentX = 0,
    dragging = false,
    didDrag = false,
    animating = false;

  function onStart(x) {
    if (animating) return;
    startX = x;
    currentX = 0;
    dragging = true;
    didDrag = false;
    filmA.style.transition = "none";
    filmB.style.transition = "none";
  }
  function onMove(x) {
    if (!dragging) return;
    currentX = x - startX;
    if (Math.abs(currentX) > 5) didDrag = true;
    // Mirror the vote animation during drag
    const t = Math.min(1, Math.abs(currentX) / 150);
    const pickedLeft = currentX < 0;
    const winner = pickedLeft ? filmA : filmB;
    const loser = pickedLeft ? filmB : filmA;
    const dir = pickedLeft ? -1 : 1;
    winner.style.transform = `translateX(${dir * t * 100}px) rotate(${dir * t * 5}deg)`;
    winner.style.opacity = `${1 - t * 0.3}`;
    loser.style.transform = `translateY(${t * 80}px) scale(${1 - t * 0.15})`;
    loser.style.opacity = `${1 - t * 0.4}`;
  }
  function onEnd() {
    if (!dragging) return;
    dragging = false;
    if (Math.abs(currentX) > 80) {
      if (currentX > 0) animateVote(true, currentPair.b.id, currentPair.a.id);
      else animateVote(false, currentPair.a.id, currentPair.b.id);
    } else {
      filmA.style.transition = "transform 0.3s ease, opacity 0.3s ease";
      filmB.style.transition = "transform 0.3s ease, opacity 0.3s ease";
      filmA.style.transform = "";
      filmA.style.opacity = "";
      filmB.style.transform = "";
      filmB.style.opacity = "";
    }
  }

  function animateVote(pickedRight, winnerId, loserId) {
    if (animating) return;
    animating = true;
    const winner = pickedRight ? filmB : filmA;
    const loser = pickedRight ? filmA : filmB;
    const dir = pickedRight ? 1 : -1;
    winner.style.transition =
      "transform 0.4s cubic-bezier(.4,0,.2,1), opacity 0.4s ease";
    loser.style.transition =
      "transform 0.4s cubic-bezier(.4,0,.2,1), opacity 0.4s ease";
    winner.style.transform = `translateX(${dir * 300}px) rotate(${dir * 12}deg)`;
    winner.style.opacity = "0";
    loser.style.transform = "translateY(200px) scale(0.7)";
    loser.style.opacity = "0";
    setTimeout(() => castVote(winnerId, loserId), 350);
  }

  const handlers = [
    [arena, "mousedown", (e) => onStart(e.clientX)],
    [window, "mousemove", (e) => onMove(e.clientX)],
    [window, "mouseup", onEnd],
    [
      arena,
      "touchstart",
      (e) => onStart(e.touches[0].clientX),
      { passive: true },
    ],
    [
      window,
      "touchmove",
      (e) => onMove(e.touches[0].clientX),
      { passive: true },
    ],
    [window, "touchend", onEnd],
    ...[
      filmA.querySelector(".poster") || filmA.querySelector(".poster-ph"),
      filmA.querySelector(".title"),
    ]
      .filter(Boolean)
      .map((el) => [
        el,
        "click",
        () => {
          if (!didDrag) animateVote(false, currentPair.a.id, currentPair.b.id);
        },
      ]),
    ...[
      filmB.querySelector(".poster") || filmB.querySelector(".poster-ph"),
      filmB.querySelector(".title"),
    ]
      .filter(Boolean)
      .map((el) => [
        el,
        "click",
        () => {
          if (!didDrag) animateVote(true, currentPair.b.id, currentPair.a.id);
        },
      ]),
    [
      btnA,
      "click",
      (e) => {
        e.stopPropagation();
        animateVote(false, currentPair.a.id, currentPair.b.id);
      },
    ],
    [
      btnB,
      "click",
      (e) => {
        e.stopPropagation();
        animateVote(true, currentPair.b.id, currentPair.a.id);
      },
    ],
    [
      document,
      "keydown",
      (e) => {
        if (e.key === "ArrowLeft")
          animateVote(false, currentPair.a.id, currentPair.b.id);
        else if (e.key === "ArrowRight")
          animateVote(true, currentPair.b.id, currentPair.a.id);
      },
    ],
  ];

  handlers.forEach(([el, evt, fn, opts]) => el.addEventListener(evt, fn, opts));
  swipeCleanup = () =>
    handlers.forEach(([el, evt, fn]) => el.removeEventListener(evt, fn));
}

async function castVote(winnerId, loserId) {
  await fetch("/api/vote", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      user_id: USER_ID,
      winner_id: winnerId,
      loser_id: loserId,
    }),
  });
  voteHistory.push({ winnerId, loserId });
  loadPair();
}

async function undoVote() {
  const last = voteHistory.pop();
  if (!last) return;
  await fetch("/api/unvote", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      user_id: USER_ID,
      winner_id: last.winnerId,
      loser_id: last.loserId,
    }),
  });
  voteCount = Math.max(0, voteCount - 1);
  const filmA = allFilms.find((f) => f.id === last.winnerId);
  const filmB = allFilms.find((f) => f.id === last.loserId);
  renderPair(filmA, filmB);
}

async function deselectAndSkip(filmId, event) {
  event.stopPropagation();
  selectedIds.delete(filmId);
  const el = document.querySelector(`.film-item[data-id="${filmId}"]`);
  if (el) el.classList.remove("selected");
  updateSelectionStatus();
  await saveSelection();
  loadPair();
}

// -- PAGE 3: Leaderboard --
async function loadLeaderboard() {
  const res = await fetch("/api/leaderboard");
  const data = await res.json();
  const container = document.getElementById("board-list");

  if (data.length === 0) {
    container.innerHTML = `
      <div class="board-empty">
        <h3>No votes yet</h3>
        <p>Start comparing films to build the leaderboard!</p>
      </div>`;
    return;
  }

  container.innerHTML = data
    .map(
      (item, i) => `
    <div class="board-item">
      <div class="board-rank">${i + 1}</div>
      ${posterImg(item.poster_url)}
      <div class="board-info">
        <div class="board-title">${esc(item.title)}</div>
        <div class="board-meta">${filmMeta(item)}</div>
      </div>
      <div class="board-stats">
        <div class="board-score">${Math.round(item.rating)}</div>
        <div class="board-record">${item.wins}W - ${item.losses}L</div>
      </div>
    </div>
  `,
    )
    .join("");
}

// -- PAGE 4: More --
async function loadMore() {
  await Promise.all([loadStats(), loadUserContributions()]);
  await Promise.all([loadUserMatrix(), loadGlobalMatrix()]);
}

async function loadStats() {
  const res = await fetch("/api/stats");
  const s = await res.json();
  const container = document.getElementById("stats-content");

  container.innerHTML = `
    <div class="stats-grid">
      ${[
        [s.total_votes, "Total Votes"],
        [s.active_users, "Active Voters"],
        [s.total_users, "Total Visitors"],
        [s.films_with_votes, "Films Voted On"],
        [s.total_films, "Total Films"],
      ]
        .map(
          ([v, l]) => `
        <div class="stat-card">
          <div class="stat-value">${v}</div>
          <div class="stat-label">${l}</div>
        </div>`,
        )
        .join("")}
    </div>
  `;
}

async function loadUserContributions() {
  const res = await fetch(`/api/user-contributions?user_id=${USER_ID}`);
  const data = await res.json();
  const container = document.getElementById("contributions-content");

  if (data.length === 0) {
    container.innerHTML = '<p class="matrix-empty">No contributions yet.</p>';
    return;
  }

  const maxVotes = Math.max(...data.map((u) => u.votes), 1);

  container.innerHTML = data
    .map(
      (u, i) => `
    <div class="contrib-item${u.is_you ? " contrib-you" : ""}">
      <div class="contrib-rank">${i + 1}</div>
      <div class="contrib-info">
        <div class="contrib-label">${esc(u.label)}</div>
        <div class="contrib-bar-track">
          <div class="contrib-bar-fill" style="width: ${((u.votes / maxVotes) * 100).toFixed(1)}%"></div>
        </div>
      </div>
      <div class="contrib-stats">
        <div class="contrib-votes">${u.votes}</div>
        <div class="contrib-detail">${u.films_selected} films</div>
      </div>
    </div>`,
    )
    .join("");
}

// -- Matrix Rendering --
function shortTitle(t) {
  return t.length > 12 ? t.slice(0, 11) + "\u2026" : t;
}

async function loadUserMatrix() {
  const res = await fetch(
    `/api/user-matrix?user_id=${USER_ID}&_=${Date.now()}`,
  );
  const data = await res.json();
  const container = document.getElementById("user-matrix");
  if (data.films.length === 0) {
    container.innerHTML =
      '<p class="matrix-empty">No votes yet. Start comparing!</p>';
    return;
  }
  container.innerHTML = renderUserMatrix(
    data.films,
    data.votes,
    data.legacy_votes,
  );
}

async function loadGlobalMatrix() {
  const res = await fetch(`/api/global-matrix?_=${Date.now()}`);
  const data = await res.json();
  const container = document.getElementById("global-matrix");
  if (data.films.length === 0) {
    container.innerHTML = '<p class="matrix-empty">No data yet.</p>';
    return;
  }
  container.innerHTML = renderGlobalMatrix(data.films, data.wins);
}

function renderUserMatrix(films, votes, legacyVotes) {
  const voteMap = new Map();
  for (const v of votes) {
    const a = Math.min(v.film_a, v.film_b),
      b = Math.max(v.film_a, v.film_b);
    voteMap.set(`${a},${b}`, v.winner);
  }
  const legacySet = new Set();
  for (const v of legacyVotes || []) {
    const a = Math.min(v.film_a, v.film_b),
      b = Math.max(v.film_a, v.film_b);
    legacySet.add(`${a},${b}`);
  }

  let html =
    '<table class="matrix-table"><thead><tr><th class="matrix-corner">\u2193 beat \u2192</th>';
  for (const f of films) {
    html += `<th class="matrix-col-header" title="${esc(f.title)}">${esc(shortTitle(f.title))}</th>`;
  }
  html += "</tr></thead><tbody>";

  for (const row of films) {
    html += `<tr><td class="matrix-row-header" title="${esc(row.title)}">${esc(shortTitle(row.title))}</td>`;
    for (const col of films) {
      if (row.id === col.id) {
        html += '<td class="matrix-cell matrix-diag"></td>';
        continue;
      }
      const a = Math.min(row.id, col.id),
        b = Math.max(row.id, col.id);
      const key = `${a},${b}`;
      const winner = voteMap.get(key);
      const isLegacy = legacySet.has(key);

      let cls = "matrix-cell",
        content = "",
        click = "";

      if (winner !== undefined) {
        if (winner === row.id) {
          cls += " matrix-win";
          content = "W";
        } else {
          cls += " matrix-loss";
          content = "L";
        }
        // Click to remove vote
        const uw = winner === row.id ? row.id : col.id;
        const ul = winner === row.id ? col.id : row.id;
        click = ` onclick="matrixUnvote(${uw},${ul})"`;
      } else if (isLegacy) {
        cls += " matrix-legacy";
        content = "?";
        click = ` onclick="matrixVote(${row.id},${col.id})"`;
      } else {
        cls += " matrix-empty-cell";
        click = ` onclick="matrixVote(${row.id},${col.id})"`;
      }

      html += `<td class="${cls}"${click} title="${esc(row.title)} vs ${esc(col.title)}">${content}</td>`;
    }
    html += "</tr>";
  }
  html += "</tbody></table>";
  return html;
}

function renderGlobalMatrix(films, wins) {
  const winMap = new Map();
  for (const w of wins) winMap.set(`${w.winner},${w.loser}`, w.count);

  let html =
    '<table class="matrix-table"><thead><tr><th class="matrix-corner">\u2193 beat \u2192</th>';
  for (const f of films) {
    html += `<th class="matrix-col-header" title="${esc(f.title)}">${esc(shortTitle(f.title))}</th>`;
  }
  html += "</tr></thead><tbody>";

  for (const row of films) {
    html += `<tr><td class="matrix-row-header" title="${esc(row.title)}">${esc(shortTitle(row.title))}</td>`;
    for (const col of films) {
      if (row.id === col.id) {
        html += '<td class="matrix-cell matrix-diag"></td>';
        continue;
      }
      const wRC = winMap.get(`${row.id},${col.id}`) || 0;
      const wCR = winMap.get(`${col.id},${row.id}`) || 0;
      const total = wRC + wCR;

      let cls = "matrix-cell";
      let content = "";
      if (total > 0) {
        const rate = wRC / total;
        if (rate > 0.5) cls += " matrix-favors-row";
        else if (rate < 0.5) cls += " matrix-favors-col";
        else cls += " matrix-neutral";
        content = `${wRC}`;
      }
      html += `<td class="${cls}" title="${esc(row.title)}: ${wRC}W / ${esc(col.title)}: ${wCR}W">${content}</td>`;
    }
    html += "</tr>";
  }
  html += "</tbody></table>";
  return html;
}

async function matrixUnvote(winnerId, loserId) {
  await fetch("/api/unvote", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      user_id: USER_ID,
      winner_id: winnerId,
      loser_id: loserId,
    }),
  });
  await Promise.all([loadUserMatrix(), loadGlobalMatrix()]);
  refreshMatrixModal();
}

async function matrixVote(winnerId, loserId) {
  await fetch("/api/vote", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      user_id: USER_ID,
      winner_id: winnerId,
      loser_id: loserId,
    }),
  });
  await Promise.all([loadUserMatrix(), loadGlobalMatrix()]);
  refreshMatrixModal();
}

// -- Fullscreen Matrix Modal --
let zoomCleanup = null;
let modalSourceId = null;

function openMatrixModal(sourceId) {
  const modal = document.getElementById("matrix-modal");
  const content = document.getElementById("matrix-modal-content");
  const source = document.getElementById(sourceId);
  if (!source) return;

  modalSourceId = sourceId;
  content.innerHTML = source.innerHTML;
  modal.classList.add("active");
  document.body.style.overflow = "hidden";
  setupPinchZoom(content);
}

function refreshMatrixModal() {
  if (!modalSourceId) return;
  const modal = document.getElementById("matrix-modal");
  if (!modal.classList.contains("active")) return;
  const source = document.getElementById(modalSourceId);
  if (!source) return;
  const content = document.getElementById("matrix-modal-content");
  content.innerHTML = source.innerHTML;
}

function closeMatrixModal() {
  const modal = document.getElementById("matrix-modal");
  modal.classList.remove("active");
  document.body.style.overflow = "";
  modalSourceId = null;
  if (zoomCleanup) {
    zoomCleanup();
    zoomCleanup = null;
  }
  const content = document.getElementById("matrix-modal-content");
  content.innerHTML = "";
}

function setupPinchZoom(container) {
  let scale = 1,
    tx = 0,
    ty = 0;
  let lastDist = 0,
    lastCX = 0,
    lastCY = 0;
  let pinching = false,
    panning = false;
  let panSX = 0,
    panSY = 0;

  function apply() {
    const inner = container.firstElementChild;
    if (inner) {
      inner.style.transform = `translate(${tx}px,${ty}px) scale(${scale})`;
      inner.style.transformOrigin = "0 0";
    }
  }

  function onTS(e) {
    if (e.touches.length === 2) {
      pinching = true;
      const dx = e.touches[0].clientX - e.touches[1].clientX;
      const dy = e.touches[0].clientY - e.touches[1].clientY;
      lastDist = Math.hypot(dx, dy);
      lastCX = (e.touches[0].clientX + e.touches[1].clientX) / 2;
      lastCY = (e.touches[0].clientY + e.touches[1].clientY) / 2;
      e.preventDefault();
    } else if (e.touches.length === 1 && scale > 1) {
      panning = true;
      panSX = e.touches[0].clientX - tx;
      panSY = e.touches[0].clientY - ty;
    }
  }
  function onTM(e) {
    if (pinching && e.touches.length === 2) {
      const dx = e.touches[0].clientX - e.touches[1].clientX;
      const dy = e.touches[0].clientY - e.touches[1].clientY;
      const dist = Math.hypot(dx, dy);
      const cx = (e.touches[0].clientX + e.touches[1].clientX) / 2;
      const cy = (e.touches[0].clientY + e.touches[1].clientY) / 2;
      scale = Math.max(0.5, Math.min(8, scale * (dist / lastDist)));
      tx += cx - lastCX;
      ty += cy - lastCY;
      lastDist = dist;
      lastCX = cx;
      lastCY = cy;
      apply();
      e.preventDefault();
    } else if (panning && e.touches.length === 1) {
      tx = e.touches[0].clientX - panSX;
      ty = e.touches[0].clientY - panSY;
      apply();
      e.preventDefault();
    }
  }
  function onTE(e) {
    if (e.touches.length < 2) pinching = false;
    if (e.touches.length < 1) panning = false;
  }
  function onWheel(e) {
    e.preventDefault();
    const rect = container.getBoundingClientRect();
    const mx = e.clientX - rect.left,
      my = e.clientY - rect.top;
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    const ns = Math.max(0.5, Math.min(8, scale * delta));
    tx = mx - (mx - tx) * (ns / scale);
    ty = my - (my - ty) * (ns / scale);
    scale = ns;
    apply();
  }

  container.addEventListener("touchstart", onTS, { passive: false });
  container.addEventListener("touchmove", onTM, { passive: false });
  container.addEventListener("touchend", onTE);
  container.addEventListener("wheel", onWheel, { passive: false });

  zoomCleanup = () => {
    container.removeEventListener("touchstart", onTS);
    container.removeEventListener("touchmove", onTM);
    container.removeEventListener("touchend", onTE);
    container.removeEventListener("wheel", onWheel);
  };
}

// -- Vote Notifications (SSE) --
function initVoteStream() {
  const es = new EventSource("/api/vote/stream");
  es.onmessage = (e) => {
    const data = JSON.parse(e.data);
    // Refresh active data pages on any vote
    if (document.getElementById("page-board").classList.contains("active")) {
      loadLeaderboard();
    }
    if (document.getElementById("page-more").classList.contains("active")) {
      loadMore();
    }
    if (data.user_id === USER_ID) return;
    const container = document.getElementById("toast-container");
    const toast = document.createElement("div");
    toast.className = "vote-toast";
    toast.innerHTML = `Someone voted: <strong>${esc(data.winner_title)}</strong> over ${esc(data.loser_title)}`;
    container.appendChild(toast);
    setTimeout(() => toast.remove(), 4000);
  };
  es.onerror = () => {
    es.close();
    setTimeout(initVoteStream, 5000);
  };
}

initVoteStream();
init().then(() => {
  let page = location.hash.slice(1);
  if (page === "stats") page = "more";
  if (page && document.getElementById(`page-${page}`)) showPage(page);
  else showPage("swipe");
});
