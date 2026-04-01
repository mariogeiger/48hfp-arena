import { esc, posterHtml, metaHtml, shortTitle, store } from "./store.js";
import { addToast, castVote, matrixAction } from "./actions.js";

// ==================== RENDERING ====================

export function render(state, prev) {
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

  // Detect rank swaps globally (before page render so toasts appear on any page)
  if (state.board !== prev.board && prev.board && prev.board.length > 0) {
    const oldRank = new Map(prev.board.map((item, i) => [item.film_id, i]));
    const notified = new Set();
    const swaps = [];
    for (let i = 0; i < state.board.length; i++) {
      const film = state.board[i];
      const oldIdx = oldRank.get(film.film_id);
      if (oldIdx === undefined || oldIdx <= i || notified.has(film.film_id))
        continue;
      const displaced = prev.board[i];
      if (displaced && !notified.has(displaced.film_id)) {
        notified.add(film.film_id);
        notified.add(displaced.film_id);
        swaps.push(
          `&#11014;&#65039; <strong>${esc(film.title)}</strong> overtook <strong>${esc(displaced.title)}</strong> → #${i + 1}`,
        );
      }
    }
    if (swaps.length) addToast(swaps.join("<br>"));
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

  if (state.films !== prev.films || state.selectedIds !== prev.selectedIds) {
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
  } else if (state.focusFilmId !== prev.focusFilmId) {
    const sel = document.getElementById("focus-film");
    if (sel) sel.value = state.focusFilmId ?? "";
  }

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
        <p>Bradley-Terry rankings based on all voter comparisons</p>
      </div>
      <div class="board-list" id="board-list"></div>
      <div class="board-export">
        <a href="/api/leaderboard.csv" target="_blank">Export as CSV</a>
      </div>`;
    page.dataset.init = "1";
  }
  if (state.board !== prev.board) {
    const list = document.getElementById("board-list");

    // FLIP step 1: snapshot old positions by film_id
    const oldPos = new Map();
    list.querySelectorAll("[data-film-id]").forEach((el) => {
      oldPos.set(el.dataset.filmId, el.getBoundingClientRect());
    });

    // Render new DOM
    list.innerHTML =
      state.board.length === 0
        ? `<div class="board-empty"><h3>No votes yet</h3><p>Start comparing films to build the leaderboard!</p></div>`
        : state.board
            .map(
              (item, i) => `
        <div class="board-item" data-film-id="${item.film_id}">
          <div class="board-rank">${i + 1}</div>
          ${posterHtml(item.poster_url)}
          <div class="board-info">
            <div class="board-title">${esc(item.title)}</div>
            <div class="board-meta">${metaHtml(item)}${item.video_url ? ` &middot; <a href="${esc(item.video_url)}" target="_blank" class="board-video">Watch</a>` : ""}</div>
          </div>
          <div class="board-stats">
            <div class="board-score">${Math.round(item.rating)}</div>
            <div class="board-record">${item.wins}W - ${item.losses}L</div>
          </div>
        </div>`,
            )
            .join("");

    // FLIP steps 2-4: invert + play
    if (oldPos.size > 0) {
      list.querySelectorAll("[data-film-id]").forEach((el) => {
        const old = oldPos.get(el.dataset.filmId);
        if (!old) return;
        const now = el.getBoundingClientRect();
        const dy = old.top - now.top;
        if (Math.abs(dy) < 1) return;
        el.style.transform = `translateY(${dy}px)`;
        el.style.transition = "none";
        // Force reflow so the browser registers the starting position
        el.offsetHeight; // eslint-disable-line no-unused-expressions
        el.style.transition = "transform 0.4s cubic-bezier(.4,0,.2,1)";
        el.style.transform = "";
      });
    }

    // Highlight changed scores
    applyChanges();

    // Re-arm for next update
    rearmBoardTracking();
  }
}

/** Capture old label->value pairs from the DOM; returns a function that,
 *  after re-render, adds `cls` to values that changed and auto-removes it. */
function trackChanges(container, itemSel, labelSel, valueSel, cls) {
  const old = {};
  container.querySelectorAll(itemSel).forEach((el) => {
    const label = el.querySelector(labelSel)?.textContent;
    const value = el.querySelector(valueSel)?.textContent;
    if (label) old[label] = value;
  });
  return () =>
    container.querySelectorAll(itemSel).forEach((el) => {
      const label = el.querySelector(labelSel)?.textContent;
      const valueEl = el.querySelector(valueSel);
      if (
        label &&
        valueEl &&
        label in old &&
        old[label] !== valueEl.textContent
      ) {
        valueEl.classList.add(cls);
        valueEl.addEventListener(
          "animationend",
          () => valueEl.classList.remove(cls),
          { once: true },
        );
      }
    });
}

let applyChanges = () => {};

function rearmBoardTracking() {
  const list = document.getElementById("board-list");
  if (list) {
    applyChanges = trackChanges(
      list,
      ".board-item",
      ".board-title",
      ".board-score",
      "score-changed",
    );
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
      <div class="reset-section">
        <button class="reset-btn" data-action="reset-votes">Reset All My Votes</button>
      </div>
      <div id="stats-content"></div>
      <div class="stats-section">
        <h3>Voter Contributions</h3>
        <div id="contributions-content"></div>
      </div>
      <div class="suggest-section">
        <h3>Suggest a Correction</h3>
        <p class="suggest-hint">Wrong title, missing video link, bad poster? Pick the film and fill in what needs changing. You can also suggest a new film.</p>
        <div class="suggest-form">
          <select id="suggest-film">
            <option value="">Pick a film...</option>
          </select>
          <input type="text" id="suggest-title" placeholder="Title">
          <input type="text" id="suggest-team" placeholder="Team">
          <input type="text" id="suggest-city" placeholder="City">
          <input type="url" id="suggest-poster" placeholder="Poster URL">
          <input type="url" id="suggest-video" placeholder="Video URL (YouTube, Vimeo...)">
          <button data-action="submit-suggestion">Open GitHub Issue</button>
        </div>
      </div>
      <div class="matrix-section">
        <h3>Your Vote Matrix</h3>
        <p class="matrix-hint">Tap empty cell to vote (row wins). Tap filled cell to remove vote.</p>
        <div class="matrix-scroll-wrapper"><div id="user-matrix"></div></div>
      </div>
      <div class="matrix-section">
        <h3>Global Win Matrix</h3>
        <p class="matrix-hint">Color = surprise: green = wins more than model expects, red = wins less. Hover for details.</p>
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

  if (
    state.films !== prev.films ||
    document.querySelectorAll("#suggest-film option").length <= 1
  ) {
    const select = document.getElementById("suggest-film");
    if (select) {
      select.innerHTML =
        `<option value="">Pick a film...</option>` +
        state.films
          .map(
            (f) =>
              `<option value="${f.id}">${esc(f.title)} &mdash; ${esc(f.team)}</option>`,
          )
          .join("");
    }
  }

  if (state.stats !== prev.stats && state.stats) {
    const s = state.stats;
    const ps = prev.stats || {};
    const bump = (val, old) =>
      val !== old && old !== undefined ? " changed" : "";
    document.getElementById("stats-content").innerHTML = `
      <div class="stats-section">
        <h3>Voting Stats</h3>
        <div class="stats-grid">
          <div class="stat-card"><div class="stat-value${bump(s.total_votes, ps.total_votes)}">${s.total_votes}</div><div class="stat-label">Total Votes</div></div>
          <div class="stat-card"><div class="stat-value${bump(s.active_users, ps.active_users)}">${s.active_users}</div><div class="stat-label">Voters</div></div>
          <div class="stat-card"><div class="stat-value${bump(s.total_films, ps.total_films)}">${s.total_films}</div><div class="stat-label">Films</div></div>
          <div class="stat-card"><div class="stat-value${bump(s.films_with_votes, ps.films_with_votes)}">${s.films_with_votes}</div><div class="stat-label">Films Voted On</div></div>
        </div>
      </div>`;
  }

  if (state.contributions !== prev.contributions) {
    const c = state.contributions;
    const pc = prev.contributions || [];
    const oldVotes = new Map(pc.map((u) => [u.label, u.votes]));
    document.getElementById("contributions-content").innerHTML =
      c.length === 0
        ? "<p>No contributions yet.</p>"
        : (() => {
            const max = c[0].votes;
            return `<div class="contrib-bars">${c
              .map((u) => {
                const changed =
                  oldVotes.has(u.label) && oldVotes.get(u.label) !== u.votes;
                const pairs = (u.films_voted * (u.films_voted - 1)) / 2;
                const pct =
                  pairs > 0 ? ((u.votes / pairs) * 100).toFixed(1) : 0;
                return `<div class="contrib-row"><span class="contrib-label">${esc(u.label)}</span><div class="contrib-bar-track"><div class="contrib-bar${u.is_you ? " contrib-you" : ""}" style="width:${(u.votes / max) * 100}%"></div></div><span class="contrib-count${changed ? " changed" : ""}" title="${u.votes} / ${pairs} possible pairs = ${pct}% coverage"><span class="contrib-num">${u.votes}</span> votes on <span class="contrib-num">${u.films_voted}</span> films</span></div>`;
              })
              .join("")}</div>`;
          })();
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
      renderMatrixCanvas(container, films, {
        cellInfo: (ri, ci) => {
          const row = films[ri],
            col = films[ci];
          const key = `${Math.min(row.id, col.id)},${Math.max(row.id, col.id)}`;
          const winner = voteMap.get(key);
          if (winner !== undefined) {
            const won = winner === row.id;
            return { bg: won ? "win" : "loss", text: won ? "W" : "L" };
          }
          if (legacySet.has(key)) return { bg: "legacy", text: "?" };
          return { bg: "empty", text: "" };
        },
        tooltip: (ri, ci) => {
          const row = films[ri],
            col = films[ci];
          return `${row.title} vs ${col.title}`;
        },
        onClick: (ri, ci) => {
          const row = films[ri],
            col = films[ci];
          const key = `${Math.min(row.id, col.id)},${Math.max(row.id, col.id)}`;
          const winner = voteMap.get(key);
          if (winner !== undefined) {
            const won = winner === row.id;
            const [w, l] = won ? [row.id, col.id] : [col.id, row.id];
            matrixAction("/api/unvote", w, l);
          } else {
            matrixAction("/api/vote", row.id, col.id);
          }
        },
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
      const scoreMap = new Map();
      for (const f of films) scoreMap.set(f.id, f.score || 1.0);
      renderMatrixCanvas(container, films, {
        cellInfo: (ri, ci) => {
          const row = films[ri],
            col = films[ci];
          const wRC = winMap.get(`${row.id},${col.id}`) || 0;
          const wCR = winMap.get(`${col.id},${row.id}`) || 0;
          const total = wRC + wCR;
          if (total === 0) return { bg: "empty", text: "", residual: 0 };
          const observed = wRC / total;
          const bi = scoreMap.get(row.id),
            bj = scoreMap.get(col.id);
          const predicted = bi / (bi + bj);
          const residual = observed - predicted;
          return { bg: "residual", text: String(wRC), residual };
        },
        tooltip: (ri, ci) => {
          const row = films[ri],
            col = films[ci];
          const wRC = winMap.get(`${row.id},${col.id}`) || 0;
          const wCR = winMap.get(`${col.id},${row.id}`) || 0;
          const total = wRC + wCR;
          if (total === 0) return `${row.title} vs ${col.title}: no votes`;
          const observed = wRC / total;
          const bi = scoreMap.get(row.id),
            bj = scoreMap.get(col.id);
          const predicted = bi / (bi + bj);
          const residual = observed - predicted;
          return `${row.title} vs ${col.title}: observed ${(observed * 100).toFixed(0)}% / model ${(predicted * 100).toFixed(0)}% (${residual > 0 ? "+" : ""}${(residual * 100).toFixed(0)}%)`;
        },
        onClick: null,
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

// ==================== Canvas Matrix ====================

const MATRIX_CELL = 20;
const MATRIX_HEADER = 70;
const MATRIX_FONT =
  "7px -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif";
const MATRIX_CELL_FONT =
  "bold 8px -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif";

function getMatrixTooltip() {
  let el = document.getElementById("matrix-tooltip");
  if (!el) {
    el = document.createElement("div");
    el.id = "matrix-tooltip";
    el.style.display = "none";
    document.body.appendChild(el);
  }
  return el;
}

function getMatrixColors() {
  const s = getComputedStyle(document.documentElement);
  return {
    bg: s.getPropertyValue("--bg").trim(),
    cardAlt: s.getPropertyValue("--card-alt").trim(),
    text: s.getPropertyValue("--text").trim(),
    textMuted: s.getPropertyValue("--text-muted").trim(),
    win: s.getPropertyValue("--win").trim(),
    winBg: s.getPropertyValue("--win-bg").trim(),
    loss: s.getPropertyValue("--loss").trim(),
    lossBg: s.getPropertyValue("--loss-bg").trim(),
    diag: s.getPropertyValue("--matrix-diag").trim(),
    neutral: s.getPropertyValue("--matrix-neutral").trim(),
    legacy: s.getPropertyValue("--matrix-legacy").trim(),
    legacyBg: s.getPropertyValue("--matrix-legacy-bg").trim(),
  };
}

function setupMatrixScroll(wrapper, contentWidth) {
  // Remove old top scrollbar if present
  const oldBar = wrapper.previousElementSibling;
  if (oldBar && oldBar.classList.contains("matrix-top-scroll")) oldBar.remove();

  // Create top scrollbar: a div with overflow-x:auto containing a spacer
  const topBar = document.createElement("div");
  topBar.className = "matrix-top-scroll";
  const spacer = document.createElement("div");
  spacer.style.width = contentWidth + "px";
  spacer.style.height = "1px";
  topBar.appendChild(spacer);
  wrapper.parentNode.insertBefore(topBar, wrapper);

  // Sync scroll positions
  let syncing = false;
  topBar.addEventListener("scroll", () => {
    if (syncing) return;
    syncing = true;
    wrapper.scrollLeft = topBar.scrollLeft;
    syncing = false;
  });
  wrapper.addEventListener("scroll", () => {
    if (syncing) return;
    syncing = true;
    topBar.scrollLeft = wrapper.scrollLeft;
    syncing = false;
  });

  // Shift+scroll → horizontal scroll
  wrapper.addEventListener("wheel", (e) => {
    if (e.shiftKey) {
      e.preventDefault();
      wrapper.scrollLeft += e.deltaY;
    }
  });
}

function renderMatrixCanvas(container, films, { cellInfo, tooltip, onClick }) {
  const n = films.length;
  const w = MATRIX_HEADER + n * MATRIX_CELL;
  const h = MATRIX_HEADER + n * MATRIX_CELL;
  const dpr = window.devicePixelRatio || 1;

  container.innerHTML = "";
  const canvas = document.createElement("canvas");
  canvas.className = "matrix-canvas";
  canvas.style.width = w + "px";
  canvas.style.height = h + "px";
  canvas.width = w * dpr;
  canvas.height = h * dpr;
  container.appendChild(canvas);

  // Top scrollbar + shift-scroll
  const wrapper = container.closest(".matrix-scroll-wrapper");
  if (wrapper) {
    setupMatrixScroll(wrapper, w);
  }

  const ctx = canvas.getContext("2d");
  ctx.scale(dpr, dpr);
  const C = getMatrixColors();

  // Background
  ctx.fillStyle = C.bg;
  ctx.fillRect(0, 0, w, h);

  // Corner label
  ctx.fillStyle = C.textMuted;
  ctx.font = MATRIX_FONT;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillText("\u2193 beat \u2192", MATRIX_HEADER / 2, MATRIX_HEADER / 2);

  // Column headers (rotated)
  ctx.save();
  ctx.font = MATRIX_FONT;
  ctx.textAlign = "left";
  ctx.textBaseline = "middle";
  ctx.fillStyle = C.textMuted;
  for (let i = 0; i < n; i++) {
    const x = MATRIX_HEADER + i * MATRIX_CELL + MATRIX_CELL / 2;
    ctx.save();
    ctx.translate(x, MATRIX_HEADER - 3);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText(shortTitle(films[i].title), 0, 0);
    ctx.restore();
  }
  ctx.restore();

  // Row headers
  ctx.font = MATRIX_FONT;
  ctx.textAlign = "right";
  ctx.textBaseline = "middle";
  ctx.fillStyle = C.textMuted;
  for (let i = 0; i < n; i++) {
    const y = MATRIX_HEADER + i * MATRIX_CELL + MATRIX_CELL / 2;
    ctx.fillText(shortTitle(films[i].title), MATRIX_HEADER - 4, y);
  }

  // Grid lines
  ctx.strokeStyle = C.cardAlt;
  ctx.lineWidth = 0.5;
  for (let i = 0; i <= n; i++) {
    const pos = MATRIX_HEADER + i * MATRIX_CELL;
    ctx.beginPath();
    ctx.moveTo(MATRIX_HEADER, pos);
    ctx.lineTo(w, pos);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(pos, MATRIX_HEADER);
    ctx.lineTo(pos, h);
    ctx.stroke();
  }

  // Cells
  const bgMap = {
    win: C.winBg,
    loss: C.lossBg,
    neutral: C.neutral,
    legacy: C.legacyBg,
    empty: null,
    diag: C.diag,
  };
  const fgMap = {
    win: C.win,
    loss: C.loss,
    neutral: C.textMuted,
    legacy: C.legacy,
    empty: C.text,
  };

  ctx.font = MATRIX_CELL_FONT;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  for (let ri = 0; ri < n; ri++) {
    for (let ci = 0; ci < n; ci++) {
      const x = MATRIX_HEADER + ci * MATRIX_CELL;
      const y = MATRIX_HEADER + ri * MATRIX_CELL;
      if (ri === ci) {
        ctx.fillStyle = C.diag;
        ctx.fillRect(x, y, MATRIX_CELL, MATRIX_CELL);
        continue;
      }
      const info = cellInfo(ri, ci);
      if (info.bg === "residual") {
        const r = info.residual || 0;
        const t = Math.min(Math.abs(r) * 3, 1);
        ctx.fillStyle =
          r > 0
            ? `rgba(39,174,96,${t * 0.6})`
            : r < 0
              ? `rgba(231,76,60,${t * 0.6})`
              : C.neutral;
        ctx.fillRect(x, y, MATRIX_CELL, MATRIX_CELL);
      } else {
        const bg = bgMap[info.bg];
        if (bg) {
          ctx.fillStyle = bg;
          ctx.fillRect(x, y, MATRIX_CELL, MATRIX_CELL);
        }
      }
      if (info.text) {
        ctx.fillStyle =
          info.bg === "residual" ? C.text : fgMap[info.bg] || C.text;
        ctx.fillText(info.text, x + MATRIX_CELL / 2, y + MATRIX_CELL / 2);
      }
    }
  }

  // Hover / click handling
  let hoverRI = -1,
    hoverCI = -1;
  const tip = getMatrixTooltip();

  function cellAt(e) {
    const rect = canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const ci = Math.floor((mx - MATRIX_HEADER) / MATRIX_CELL);
    const ci2 = Math.floor((my - MATRIX_HEADER) / MATRIX_CELL);
    if (ci >= 0 && ci < n && ci2 >= 0 && ci2 < n && ci !== ci2)
      return [ci2, ci];
    return null;
  }

  canvas.addEventListener("mousemove", (e) => {
    const cell = cellAt(e);
    if (!cell) {
      tip.style.display = "none";
      canvas.style.cursor = "default";
      hoverRI = hoverCI = -1;
      return;
    }
    const [ri, ci] = cell;
    if (ri !== hoverRI || ci !== hoverCI) {
      hoverRI = ri;
      hoverCI = ci;
      tip.textContent = tooltip(ri, ci);
      canvas.style.cursor = onClick ? "pointer" : "default";
    }
    tip.style.display = "";
    tip.style.left = e.pageX + 12 + "px";
    tip.style.top = e.pageY + 12 + "px";
  });

  canvas.addEventListener("mouseleave", () => {
    tip.style.display = "none";
    hoverRI = hoverCI = -1;
  });

  if (onClick) {
    canvas.addEventListener("click", (e) => {
      const cell = cellAt(e);
      if (cell) onClick(cell[0], cell[1]);
    });
  }
}

// ==================== Toasts ====================

function renderToasts(state) {
  const container = document.getElementById("toast-container");
  container.querySelectorAll("[data-toast-id]").forEach((el) => {
    if (!state.toasts.find((t) => String(t.id) === el.dataset.toastId))
      el.remove();
  });
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

// ==================== Swipe Controller ====================

export function createSwipeController() {
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

  window.addEventListener("pointerdown", onDown);
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);

  document.addEventListener("keydown", (e) => {
    if (store.get().page !== "swipe") return;
    if (e.key === "ArrowLeft") pickSide(false);
    else if (e.key === "ArrowRight") pickSide(true);
  });

  return { pickSide, wasDrag: () => didDrag };
}
