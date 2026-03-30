// ==================== CONSTANTS ====================

const USER_ID =
  localStorage.getItem("filmrank_uid") ||
  (() => {
    const id = crypto.randomUUID();
    localStorage.setItem("filmrank_uid", id);
    return id;
  })();

// ==================== UTILITIES ====================

function esc(s) {
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}

function api(path, body) {
  const opts = body
    ? {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      }
    : {};
  return fetch(path, opts).then((r) => r.json());
}

const brokenPosters = new Set();

function posterHtml(url) {
  if (!url || brokenPosters.has(url))
    return `<div class="poster-ph">&#127902;</div>`;
  return `<img class="poster" src="${esc(url)}">`;
}

function metaHtml(f) {
  return `${esc(f.team)}${f.city ? " &middot; " + esc(f.city) : ""}`;
}

function shortTitle(t) {
  return t.length > 12 ? t.slice(0, 11) + "\u2026" : t;
}

// ==================== STORE ====================

function createStore(initial) {
  let state = initial;
  let listener = null;
  return {
    get: () => state,
    set(fn) {
      const prev = state;
      state = fn(state);
      if (state !== prev && listener) listener(state, prev);
    },
    subscribe(fn) {
      listener = fn;
    },
  };
}

const store = createStore({
  page: "swipe",
  films: [],
  selectedIds: new Set(
    JSON.parse(localStorage.getItem("filmrank_selected") || "[]"),
  ),
  searchQuery: "",

  pair: null,
  pairDone: false,
  pairDoneReason: "",
  voteCount: 0,
  voteHistory: [],
  focusFilmId: null,

  board: [],
  stats: null,
  contributions: [],
  userMatrix: null,
  globalMatrix: null,

  toasts: [],
});

// ==================== ACTIONS ====================

let saveTimeout = null;

function navigate(page) {
  store.set((s) => ({ ...s, page }));
  document.body.classList.toggle("swipe-active", page === "swipe");
  location.hash = page;
  const loaders = { swipe: loadPair, board: loadBoard, more: loadMore };
  if (loaders[page]) loaders[page]();
}

function toggleFilm(id) {
  store.set((s) => {
    const next = new Set(s.selectedIds);
    next.has(id) ? next.delete(id) : next.add(id);
    return { ...s, selectedIds: next };
  });
  debounceSave();
}

function selectAll() {
  store.set((s) => {
    const next = new Set(s.selectedIds);
    const q = s.searchQuery.toLowerCase();
    const visible = q
      ? s.films.filter((f) =>
          [f.title, f.team, f.city].some((t) => t.toLowerCase().includes(q)),
        )
      : s.films;
    visible.forEach((f) => next.add(f.id));
    return { ...s, selectedIds: next };
  });
  debounceSave();
}

function deselectAll() {
  store.set((s) => ({ ...s, selectedIds: new Set() }));
  debounceSave();
}

function setSearch(query) {
  store.set((s) => ({ ...s, searchQuery: query }));
}

function debounceSave() {
  clearTimeout(saveTimeout);
  saveTimeout = setTimeout(saveSelection, 500);
}

async function saveSelection() {
  const ids = [...store.get().selectedIds];
  localStorage.setItem("filmrank_selected", JSON.stringify(ids));
  await api("/api/selection", { user_id: USER_ID, film_ids: ids });
}

function setFocusFilm(id) {
  store.set((s) => ({ ...s, focusFilmId: id }));
  loadPair();
}

async function loadPair() {
  const { focusFilmId } = store.get();
  let url = `/api/pair?user_id=${USER_ID}`;
  if (focusFilmId) url += `&focus_film=${focusFilmId}`;
  const data = await api(url);

  if (data.done) {
    const { selectedIds, focusFilmId: fid } = store.get();
    const reason =
      selectedIds.size < 2 ? "not_enough" : fid ? "focus_done" : "all_done";
    store.set((s) => ({
      ...s,
      pair: null,
      pairDone: true,
      pairDoneReason: reason,
      voteCount: data.votes || 0,
    }));
  } else {
    store.set((s) => ({
      ...s,
      pair: { a: data.a, b: data.b },
      pairDone: false,
      voteCount: data.votes || 0,
    }));
  }
}

async function castVote(winnerId, loserId) {
  await api("/api/vote", {
    user_id: USER_ID,
    winner_id: winnerId,
    loser_id: loserId,
  });
  store.set((s) => ({
    ...s,
    voteHistory: [...s.voteHistory, { winnerId, loserId }],
  }));
  loadPair();
}

async function undoVote() {
  const { voteHistory, films } = store.get();
  if (!voteHistory.length) return;
  const last = voteHistory[voteHistory.length - 1];
  await api("/api/unvote", {
    user_id: USER_ID,
    winner_id: last.winnerId,
    loser_id: last.loserId,
  });
  const a = films.find((f) => f.id === last.winnerId);
  const b = films.find((f) => f.id === last.loserId);
  store.set((s) => ({
    ...s,
    voteHistory: s.voteHistory.slice(0, -1),
    voteCount: Math.max(0, s.voteCount - 1),
    pair: a && b ? { a, b } : s.pair,
    pairDone: false,
  }));
}

async function deselectAndSkip(filmId) {
  store.set((s) => {
    const next = new Set(s.selectedIds);
    next.delete(filmId);
    return { ...s, selectedIds: next };
  });
  await saveSelection();
  loadPair();
}

async function loadBoard() {
  const data = await api("/api/leaderboard");
  store.set((s) => ({ ...s, board: data }));
}

async function loadMore() {
  const [stats, contributions] = await Promise.all([
    api("/api/stats"),
    api(`/api/user-contributions?user_id=${USER_ID}`),
  ]);
  store.set((s) => ({ ...s, stats, contributions }));
  const [userMatrix, globalMatrix] = await Promise.all([
    api(`/api/user-matrix?user_id=${USER_ID}&_=${Date.now()}`),
    api(`/api/global-matrix?_=${Date.now()}`),
  ]);
  store.set((s) => ({ ...s, userMatrix, globalMatrix }));
}

async function matrixAction(endpoint, w, l) {
  await api(endpoint, { user_id: USER_ID, winner_id: w, loser_id: l });
  const [userMatrix, globalMatrix] = await Promise.all([
    api(`/api/user-matrix?user_id=${USER_ID}&_=${Date.now()}`),
    api(`/api/global-matrix?_=${Date.now()}`),
  ]);
  store.set((s) => ({ ...s, userMatrix, globalMatrix }));
}

function addToast(html) {
  const id = Date.now() + Math.random();
  store.set((s) => ({ ...s, toasts: [...s.toasts, { id, html }] }));
  setTimeout(
    () =>
      store.set((s) => ({
        ...s,
        toasts: s.toasts.filter((t) => t.id !== id),
      })),
    4000,
  );
}

// ==================== RENDERING ====================

function render(state, prev) {
  // Nav
  if (state.page !== prev.page) {
    document
      .querySelectorAll(".page")
      .forEach((p) => p.classList.remove("active"));
    document.getElementById(`page-${state.page}`).classList.add("active");
    document.querySelectorAll(".nav button[data-action]").forEach((b) => {
      b.classList.toggle("active", b.dataset.page === state.page);
    });
  }

  // Active page
  const renderers = {
    select: renderSelect,
    swipe: renderSwipe,
    board: renderBoard,
    more: renderMore,
  };
  renderers[state.page](state, prev);

  // Toasts
  if (state.toasts !== prev.toasts) renderToasts(state);
}

// -- Select Page --

function renderSelect(state, prev) {
  const page = document.getElementById("page-select");
  let needsList = false;
  if (!page.dataset.init) {
    page.innerHTML = `
      <div class="select-header">
        <p>Select the films you remember, then compare them head-to-head.</p>
        <div class="search-wrapper">
          <input class="search-box" type="text" placeholder="Search films, teams, cities...">
          <button class="search-clear" data-action="clear-search">&times;</button>
        </div>
        <div class="select-actions">
          <button data-action="select-all">Select All</button>
          <button data-action="deselect-all">Deselect All</button>
        </div>
      </div>
      <div class="film-list" id="film-list"></div>
      <div class="selection-status" id="selection-status"></div>`;
    page.dataset.init = "1";
    needsList = true;
  }

  if (
    needsList ||
    state.films !== prev.films ||
    state.searchQuery !== prev.searchQuery ||
    state.selectedIds !== prev.selectedIds
  ) {
    const q = state.searchQuery.toLowerCase();
    const visible = q
      ? state.films.filter((f) =>
          [f.title, f.team, f.city].some((s) => s.toLowerCase().includes(q)),
        )
      : state.films;
    document.getElementById("film-list").innerHTML = visible
      .map(
        (f) => `
      <div class="film-item ${state.selectedIds.has(f.id) ? "selected" : ""}"
           data-action="toggle-film" data-id="${f.id}">
        <div class="film-check"></div>
        ${posterHtml(f.poster_url)}
        <div class="film-info">
          <div class="film-title">${esc(f.title)}</div>
          <div class="film-meta">${metaHtml(f)}</div>
        </div>
      </div>`,
      )
      .join("");
  }

  const n = state.selectedIds.size;
  const status = document.getElementById("selection-status");
  if (status) {
    status.textContent =
      n < 2 ? "Select at least 2 films to compare" : `${n} films selected`;
    status.classList.toggle("has-enough", n >= 2);
  }
}

// -- Swipe Page --

function renderSwipe(state, prev) {
  const page = document.getElementById("page-swipe");
  if (!page.dataset.init) {
    page.innerHTML = `
      <div id="focus-picker-container"></div>
      <div class="swipe-container" id="swipe-container"></div>`;
    page.dataset.init = "1";
  }

  // Focus picker
  if (
    state.films !== prev.films ||
    state.selectedIds !== prev.selectedIds ||
    state.focusFilmId !== prev.focusFilmId
  ) {
    const options = state.films
      .filter((f) => state.selectedIds.has(f.id))
      .map(
        (f) =>
          `<option value="${f.id}" ${f.id === state.focusFilmId ? "selected" : ""}>${esc(f.title)}</option>`,
      )
      .join("");
    document.getElementById("focus-picker-container").innerHTML = `
      <div class="focus-picker">
        <label for="focus-film">Compare against:</label>
        <select id="focus-film">
          <option value="">Random pairs</option>
          ${options}
        </select>
      </div>`;
  }

  // Pair or done state — full re-render only when pair changes
  if (state.pair !== prev.pair || state.pairDone !== prev.pairDone) {
    const container = document.getElementById("swipe-container");
    if (state.pairDone) {
      const r = state.pairDoneReason;
      const actionMap = {
        not_enough: ["show-select", "Select Films"],
        focus_done: ["clear-focus", "Compare All"],
        all_done: ["show-board", "View Leaderboard"],
      };
      const [action, label] = actionMap[r] || actionMap.all_done;
      const msg =
        r === "not_enough"
          ? "Go back and pick at least 2 films you've watched."
          : r === "focus_done"
            ? "You've compared this film against all others."
            : `You've compared all possible pairs. (${state.voteCount} votes cast)`;
      container.innerHTML = `
        <div class="swipe-done">
          <h2>${r === "not_enough" ? "Select films first" : "All done!"}</h2>
          <p>${msg}</p>
          <button data-action="${action}">${label}</button>
        </div>`;
    } else if (state.pair) {
      const { a, b } = state.pair;
      container.innerHTML = `
        <div>
          <div class="vs-badge">VS</div>
          <div class="swipe-arena" id="swipe-arena">
            <div class="film-card" id="film-a" data-action="pick-a">
              ${posterHtml(a.poster_url)}
              <div class="title">${esc(a.title)}</div>
              <div class="meta">${metaHtml(a)}</div>
              <button class="deselect-btn" data-action="deselect-skip" data-id="${a.id}">Haven't seen it</button>
            </div>
            <div class="film-card" id="film-b" data-action="pick-b">
              ${posterHtml(b.poster_url)}
              <div class="title">${esc(b.title)}</div>
              <div class="meta">${metaHtml(b)}</div>
              <button class="deselect-btn" data-action="deselect-skip" data-id="${b.id}">Haven't seen it</button>
            </div>
          </div>
          <div class="swipe-buttons">
            <button class="swipe-arrow-btn" data-action="pick-a" title="Pick left">&larr;</button>
            <button class="swipe-arrow-btn" data-action="pick-b" title="Pick right">&rarr;</button>
          </div>
          <div class="swipe-progress">${state.voteCount} comparisons made</div>
          <div class="swipe-bottom-actions">
            <button class="undo-btn" data-action="undo" ${state.voteHistory.length ? "" : "disabled"}>Undo</button>
            <button class="skip-btn" data-action="skip">Skip</button>
          </div>
        </div>`;
    }
  }

  // Targeted updates (no full re-render)
  if (state.voteCount !== prev.voteCount) {
    const el = document.querySelector(".swipe-progress");
    if (el) el.textContent = `${state.voteCount} comparisons made`;
  }
  if (state.voteHistory !== prev.voteHistory) {
    const el = document.querySelector(".undo-btn");
    if (el) el.disabled = !state.voteHistory.length;
  }
}

// -- Board Page --

function renderBoard(state, prev) {
  const page = document.getElementById("page-board");
  if (!page.dataset.init) {
    page.innerHTML = `
      <div class="page-header">
        <h1>Leaderboard</h1>
        <p>Bradley-Terry rankings based on all user votes</p>
      </div>
      <div class="board-list" id="board-list"></div>
      <div class="board-export">
        <a href="/api/leaderboard.csv" target="_blank">Export as CSV</a>
      </div>`;
    page.dataset.init = "1";
  }
  if (state.board !== prev.board) {
    document.getElementById("board-list").innerHTML =
      state.board.length === 0
        ? `<div class="board-empty"><h3>No votes yet</h3><p>Start comparing films to build the leaderboard!</p></div>`
        : state.board
            .map(
              (item, i) => `
        <div class="board-item">
          <div class="board-rank">${i + 1}</div>
          ${posterHtml(item.poster_url)}
          <div class="board-info">
            <div class="board-title">${esc(item.title)}</div>
            <div class="board-meta">${metaHtml(item)}</div>
          </div>
          <div class="board-stats">
            <div class="board-score">${Math.round(item.rating)}</div>
            <div class="board-record">${item.wins}W - ${item.losses}L</div>
          </div>
        </div>`,
            )
            .join("");
  }
}

// -- More Page --

function renderMore(state, prev) {
  const page = document.getElementById("page-more");
  if (!page.dataset.init) {
    page.innerHTML = `
      <div class="page-header">
        <h1>More</h1>
        <p>Voting stats, your votes matrix, and global results</p>
      </div>
      <div id="stats-content"></div>
      <div class="stats-section">
        <h3>User Contributions</h3>
        <div id="contributions-content"></div>
      </div>
      <div class="matrix-section">
        <h3>Your Vote Matrix</h3>
        <p class="matrix-hint">Tap empty cell to vote (row wins). Tap filled cell to remove vote.</p>
        <div class="matrix-scroll-wrapper"><div id="user-matrix"></div></div>
      </div>
      <div class="matrix-section">
        <h3>Global Win Matrix</h3>
        <p class="matrix-hint">Aggregate wins across all voters. Read only.</p>
        <div class="matrix-scroll-wrapper"><div id="global-matrix"></div></div>
      </div>
      <div class="about-section">
        <h3>How It Works</h3>
        <div class="about-content">
          <p>
            Each head-to-head vote feeds a
            <a href="https://en.wikipedia.org/wiki/Bradley%E2%80%93Terry_model" target="_blank">Bradley&ndash;Terry model</a>,
            a statistical method for ranking items from pairwise comparisons.
            Every film gets a strength parameter &beta;. The probability that
            film&nbsp;A beats film&nbsp;B is simply
            &beta;<sub>A</sub>&thinsp;/&thinsp;(&beta;<sub>A</sub>&nbsp;+&nbsp;&beta;<sub>B</sub>).
          </p>
          <p>
            Strengths are estimated using the
            <a href="https://en.wikipedia.org/wiki/MM_algorithm" target="_blank">MM&nbsp;algorithm</a>
            (minorization&ndash;maximization), which iterates until convergence.
            Films with zero wins are pinned to a near-zero score.
            The displayed score is <code>500 &times; log<sub>2</sub>(1 + &beta;)</code>,
            mapping the raw strength to a human-friendly number.
          </p>
          <p>
            Pairs are not presented randomly. The system uses
            <a href="https://en.wikipedia.org/wiki/Optimal_design#D-optimality" target="_blank">D-optimal experimental design</a>
            based on the Fisher Information matrix to pick the most informative
            pair next &mdash; prioritizing matchups between closely-ranked films
            and films with fewer comparisons. This means your votes reduce
            uncertainty as fast as possible.
          </p>
          <p>
            A film appears on the leaderboard once it has at least 10 comparisons
            from at least 2 different voters. All votes from all users are
            aggregated into one global ranking.
          </p>
        </div>
      </div>`;
    page.dataset.init = "1";
  }

  if (state.stats !== prev.stats && state.stats) {
    const s = state.stats;
    document.getElementById("stats-content").innerHTML = `
      <div class="stats-grid">
        ${[
          [s.total_votes, "Total Votes"],
          [s.active_users, "Active Voters"],
          [s.total_users, "Total Visitors"],
          [s.films_with_votes, "Films Voted On"],
          [s.total_films, "Total Films"],
        ]
          .map(
            ([v, l]) =>
              `<div class="stat-card"><div class="stat-value">${v}</div><div class="stat-label">${l}</div></div>`,
          )
          .join("")}
      </div>`;
  }

  if (state.contributions !== prev.contributions) {
    const data = state.contributions;
    const container = document.getElementById("contributions-content");
    if (data.length === 0) {
      container.innerHTML = '<p class="matrix-empty">No contributions yet.</p>';
    } else {
      const max = Math.max(...data.map((u) => u.votes), 1);
      container.innerHTML = data
        .map(
          (u, i) => `
        <div class="contrib-item${u.is_you ? " contrib-you" : ""}">
          <div class="contrib-rank">${i + 1}</div>
          <div class="contrib-info">
            <div class="contrib-label">${esc(u.label)}</div>
            <div class="contrib-bar-track">
              <div class="contrib-bar-fill" style="width: ${((u.votes / max) * 100).toFixed(1)}%"></div>
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
  }

  if (state.userMatrix !== prev.userMatrix || state.board !== prev.board) {
    const container = document.getElementById("user-matrix");
    const data = state.userMatrix;
    if (!data || data.films.length === 0) {
      container.innerHTML =
        '<p class="matrix-empty">No votes yet. Start comparing!</p>';
    } else {
      const films = sortFilmsByBoard(data.films, state.board);
      const voteMap = new Map();
      for (const v of data.votes) {
        const [a, b] = [
          Math.min(v.film_a, v.film_b),
          Math.max(v.film_a, v.film_b),
        ];
        voteMap.set(`${a},${b}`, v.winner);
      }
      const legacySet = new Set();
      for (const v of data.legacy_votes || []) {
        const [a, b] = [
          Math.min(v.film_a, v.film_b),
          Math.max(v.film_a, v.film_b),
        ];
        legacySet.add(`${a},${b}`);
      }
      container.innerHTML = renderMatrixTable(films, (row, col) => {
        const key = `${Math.min(row.id, col.id)},${Math.max(row.id, col.id)}`;
        const winner = voteMap.get(key);
        const title = `${esc(row.title)} vs ${esc(col.title)}`;
        if (winner !== undefined) {
          const won = winner === row.id;
          const [w, l] = won ? [row.id, col.id] : [col.id, row.id];
          return `<td class="matrix-cell ${won ? "matrix-win" : "matrix-loss"}" data-action="matrix-unvote" data-w="${w}" data-l="${l}" title="${title}">${won ? "W" : "L"}</td>`;
        }
        if (legacySet.has(key))
          return `<td class="matrix-cell matrix-legacy" data-action="matrix-vote" data-w="${row.id}" data-l="${col.id}" title="${title}">?</td>`;
        return `<td class="matrix-cell matrix-empty-cell" data-action="matrix-vote" data-w="${row.id}" data-l="${col.id}" title="${title}"></td>`;
      });
    }
  }

  if (state.globalMatrix !== prev.globalMatrix || state.board !== prev.board) {
    const container = document.getElementById("global-matrix");
    const data = state.globalMatrix;
    if (!data || data.films.length === 0) {
      container.innerHTML = '<p class="matrix-empty">No data yet.</p>';
    } else {
      const films = sortFilmsByBoard(data.films, state.board);
      const winMap = new Map();
      for (const w of data.wins) winMap.set(`${w.winner},${w.loser}`, w.count);
      container.innerHTML = renderMatrixTable(films, (row, col) => {
        const wRC = winMap.get(`${row.id},${col.id}`) || 0;
        const wCR = winMap.get(`${col.id},${row.id}`) || 0;
        const total = wRC + wCR;
        let cls = "matrix-cell";
        if (total > 0) {
          const rate = wRC / total;
          cls +=
            rate > 0.5
              ? " matrix-favors-row"
              : rate < 0.5
                ? " matrix-favors-col"
                : " matrix-neutral";
        }
        return `<td class="${cls}" title="${esc(row.title)}: ${wRC}W / ${esc(col.title)}: ${wCR}W">${total > 0 ? wRC : ""}</td>`;
      });
    }
  }
}

function sortFilmsByBoard(films, board) {
  const rank = new Map(board.map((b, i) => [b.film_id, i]));
  return [...films].sort(
    (a, b) => (rank.get(a.id) ?? Infinity) - (rank.get(b.id) ?? Infinity),
  );
}

function renderMatrixTable(films, cellFn) {
  const headers = films
    .map(
      (f) =>
        `<th class="matrix-col-header" title="${esc(f.title)}">${esc(shortTitle(f.title))}</th>`,
    )
    .join("");
  const rows = films
    .map((row) => {
      const cells = films
        .map((col) =>
          row.id === col.id
            ? '<td class="matrix-cell matrix-diag"></td>'
            : cellFn(row, col),
        )
        .join("");
      return `<tr><td class="matrix-row-header" title="${esc(row.title)}">${esc(shortTitle(row.title))}</td>${cells}</tr>`;
    })
    .join("");
  return `<table class="matrix-table"><thead><tr><th class="matrix-corner">\u2193 beat \u2192</th>${headers}</tr></thead><tbody>${rows}</tbody></table>`;
}

// -- Toasts --

function renderToasts(state) {
  const container = document.getElementById("toast-container");
  // Remove toasts not in state
  container.querySelectorAll("[data-toast-id]").forEach((el) => {
    if (!state.toasts.find((t) => String(t.id) === el.dataset.toastId))
      el.remove();
  });
  // Add new toasts
  for (const t of state.toasts) {
    if (!container.querySelector(`[data-toast-id="${t.id}"]`)) {
      const el = document.createElement("div");
      el.className = "vote-toast";
      el.dataset.toastId = t.id;
      el.innerHTML = t.html;
      container.appendChild(el);
    }
  }
}

// ==================== SWIPE CONTROLLER ====================

function createSwipeController() {
  let startX = 0,
    dx = 0,
    dragging = false,
    didDrag = false,
    animating = false;

  function getCards() {
    return [
      document.getElementById("film-a"),
      document.getElementById("film-b"),
    ];
  }

  function onDown(e) {
    if (animating || store.get().page !== "swipe") return;
    if (!e.target.closest("#swipe-arena")) return;
    if (e.target.closest(".deselect-btn")) return;
    startX = e.clientX;
    dx = 0;
    dragging = true;
    didDrag = false;
    const [a, b] = getCards();
    if (a) a.style.transition = "none";
    if (b) b.style.transition = "none";
  }

  function onMove(e) {
    if (!dragging) return;
    dx = e.clientX - startX;
    if (Math.abs(dx) > 5) didDrag = true;
    const [filmA, filmB] = getCards();
    if (!filmA || !filmB) return;
    const t = Math.min(1, Math.abs(dx) / 150);
    const left = dx < 0;
    const [winner, loser] = left ? [filmA, filmB] : [filmB, filmA];
    const dir = left ? -1 : 1;
    winner.style.transform = `translateX(${dir * t * 100}px) rotate(${dir * t * 5}deg)`;
    winner.style.opacity = `${1 - t * 0.3}`;
    loser.style.transform = `translateY(${t * 80}px) scale(${1 - t * 0.15})`;
    loser.style.opacity = `${1 - t * 0.4}`;
  }

  function onUp() {
    if (!dragging) return;
    dragging = false;
    if (Math.abs(dx) > 80) {
      pickSide(dx > 0);
    } else {
      const [filmA, filmB] = getCards();
      const ease = "transform 0.3s ease, opacity 0.3s ease";
      if (filmA) {
        filmA.style.transition = ease;
        filmA.style.transform = "";
        filmA.style.opacity = "";
      }
      if (filmB) {
        filmB.style.transition = ease;
        filmB.style.transform = "";
        filmB.style.opacity = "";
      }
    }
  }

  function pickSide(right) {
    const { pair } = store.get();
    if (!pair || animating) return;
    animating = true;
    const [filmA, filmB] = getCards();
    if (!filmA || !filmB) {
      animating = false;
      return;
    }
    const [winner, loser] = right ? [filmB, filmA] : [filmA, filmB];
    const dir = right ? 1 : -1;
    const ease = "transform 0.4s cubic-bezier(.4,0,.2,1), opacity 0.4s ease";
    winner.style.transition = loser.style.transition = ease;
    winner.style.transform = `translateX(${dir * 300}px) rotate(${dir * 12}deg)`;
    winner.style.opacity = "0";
    loser.style.transform = "translateY(200px) scale(0.7)";
    loser.style.opacity = "0";
    const [wId, lId] = right ? [pair.b.id, pair.a.id] : [pair.a.id, pair.b.id];
    setTimeout(() => {
      animating = false;
      castVote(wId, lId);
    }, 350);
  }

  // Pointer events (unified mouse + touch)
  window.addEventListener("pointerdown", onDown);
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);

  // Keyboard
  document.addEventListener("keydown", (e) => {
    if (store.get().page !== "swipe") return;
    if (e.key === "ArrowLeft") pickSide(false);
    else if (e.key === "ArrowRight") pickSide(true);
  });

  return { pickSide, wasDrag: () => didDrag };
}

// ==================== EVENT DELEGATION ====================

let swipeCtrl;

function setupEvents() {
  swipeCtrl = createSwipeController();

  const actions = {
    navigate: (el) => navigate(el.dataset.page),
    "toggle-film": (el) => toggleFilm(parseInt(el.dataset.id)),
    "select-all": () => selectAll(),
    "deselect-all": () => deselectAll(),
    "clear-search": () => {
      const input = document.querySelector(".search-box");
      if (input) input.value = "";
      setSearch("");
    },
    "pick-a": (el, e) => {
      if (el.closest("#swipe-arena") && swipeCtrl.wasDrag()) return;
      e.stopPropagation();
      swipeCtrl.pickSide(false);
    },
    "pick-b": (el, e) => {
      if (el.closest("#swipe-arena") && swipeCtrl.wasDrag()) return;
      e.stopPropagation();
      swipeCtrl.pickSide(true);
    },
    "deselect-skip": (el, e) => {
      e.stopPropagation();
      deselectAndSkip(parseInt(el.dataset.id));
    },
    undo: () => undoVote(),
    skip: () => loadPair(),
    "show-select": () => navigate("select"),
    "show-board": () => navigate("board"),
    "clear-focus": () => setFocusFilm(null),
    "matrix-vote": (el) =>
      matrixAction("/api/vote", parseInt(el.dataset.w), parseInt(el.dataset.l)),
    "matrix-unvote": (el) =>
      matrixAction(
        "/api/unvote",
        parseInt(el.dataset.w),
        parseInt(el.dataset.l),
      ),
  };

  document.body.addEventListener("click", (e) => {
    const el = e.target.closest("[data-action]");
    if (!el) return;
    const handler = actions[el.dataset.action];
    if (handler) handler(el, e);
  });

  document.body.addEventListener("input", (e) => {
    if (e.target.matches(".search-box")) setSearch(e.target.value);
  });

  document.body.addEventListener("change", (e) => {
    if (e.target.matches("#focus-film"))
      setFocusFilm(e.target.value ? parseInt(e.target.value) : null);
  });

  // Poster error handling (capture phase)
  document.body.addEventListener(
    "error",
    (e) => {
      if (e.target.tagName === "IMG" && e.target.classList.contains("poster")) {
        brokenPosters.add(e.target.src);
        const ph = document.createElement("div");
        ph.className = "poster-ph";
        ph.innerHTML = "&#127902;";
        e.target.replaceWith(ph);
      }
    },
    true,
  );
}

// ==================== SSE ====================

function initVoteStream() {
  const es = new EventSource("/api/vote/stream");
  es.onmessage = (e) => {
    const data = JSON.parse(e.data);
    const page = store.get().page;
    if (page === "board") loadBoard();
    if (page === "more") loadMore();
    if (data.user_id === USER_ID) return;
    addToast(
      `Someone voted: <strong>${esc(data.winner_title)}</strong> over ${esc(data.loser_title)}`,
    );
  };
  es.onerror = () => {
    es.close();
    setTimeout(initVoteStream, 5000);
  };
}

// ==================== BOOT ====================

setupEvents();
store.subscribe(render);
initVoteStream();

(async function init() {
  const [films, board] = await Promise.all([
    api("/api/films"),
    api("/api/leaderboard"),
  ]);

  // Preload posters
  films.forEach((f) => {
    if (!f.poster_url) return;
    const img = new Image();
    img.onerror = () => brokenPosters.add(f.poster_url);
    img.src = f.poster_url;
  });

  // Sort by leaderboard rank
  const rankById = new Map(board.map((item, i) => [item.film_id, i]));
  films.sort(
    (a, b) =>
      (rankById.get(a.id) ?? Infinity) - (rankById.get(b.id) ?? Infinity),
  );

  // Default: select all if none selected
  const s = store.get();
  let selectedIds = s.selectedIds;
  if (selectedIds.size === 0) {
    selectedIds = new Set(films.map((f) => f.id));
  }

  store.set(() => ({ ...s, films, board, selectedIds }));

  if (selectedIds.size >= 2) await saveSelection();

  // Route
  let page = location.hash.slice(1);
  if (page === "stats") page = "more";
  if (page && document.getElementById(`page-${page}`)) navigate(page);
  else navigate("swipe");
})();
