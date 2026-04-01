import { USER_ID, api, esc, brokenPosters, store } from "./store.js";
import {
  navigate,
  toggleFilm,
  selectAll,
  deselectAll,
  setSearch,
  setFocusFilm,
  loadPair,
  undoVote,
  deselectAndSkip,
  loadBoard,
  loadMore,
  matrixAction,
  resetVotes,
  submitSuggestion,
  addToast,
  saveSelection,
} from "./actions.js";
import { render, createSwipeController } from "./render.js";

// ==================== EVENT DELEGATION ====================

const swipeCtrl = createSwipeController();

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
    matrixAction("/api/unvote", parseInt(el.dataset.w), parseInt(el.dataset.l)),
  "reset-votes": () => resetVotes(),
  "submit-suggestion": () => submitSuggestion(),
  "welcome-promise": () => {
    localStorage.setItem("filmrank_promised", "1");
    document.getElementById("welcome-overlay").classList.add("hidden");
  },
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
  if (e.target.matches("#suggest-film")) {
    const film = store
      .get()
      .films.find((f) => f.id === parseInt(e.target.value));
    document.getElementById("suggest-title").value = film ? film.title : "";
    document.getElementById("suggest-team").value = film ? film.team : "";
    document.getElementById("suggest-city").value = film ? film.city : "";
    document.getElementById("suggest-poster").value = film
      ? film.poster_url
      : "";
    document.getElementById("suggest-video").value = film
      ? film.video_url || ""
      : "";
  }
});

// Poster error handling (capture phase)
document.body.addEventListener(
  "error",
  (e) => {
    if (e.target.tagName === "IMG" && e.target.classList.contains("poster")) {
      brokenPosters.add(e.target.src);
      const ph = document.createElement("div");
      ph.className = "poster-ph";
      ph.innerHTML = "&#127916;";
      e.target.replaceWith(ph);
    }
  },
  true,
);

// ==================== SSE ====================

function initVoteStream() {
  const es = new EventSource("/api/vote/stream");
  es.onmessage = (e) => {
    const data = JSON.parse(e.data);
    const page = store.get().page;
    loadBoard();
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

store.subscribe(render);
initVoteStream();

if (localStorage.getItem("filmrank_promised")) {
  document.getElementById("welcome-overlay").classList.add("hidden");
}

(async function init() {
  const [films, board] = await Promise.all([
    api("/api/films"),
    api("/api/leaderboard"),
  ]);

  films.forEach((f) => {
    if (!f.poster_url) return;
    const img = new Image();
    img.onerror = () => brokenPosters.add(f.poster_url);
    img.src = f.poster_url;
  });

  const rankById = new Map(board.map((item, i) => [item.film_id, i]));
  films.sort(
    (a, b) =>
      (rankById.get(a.id) ?? Infinity) - (rankById.get(b.id) ?? Infinity),
  );

  const s = store.get();
  store.set(() => ({ ...s, films, board }));

  if (s.selectedIds.size >= 2) await saveSelection();

  let page = location.hash.slice(1);
  if (page === "stats") page = "more";
  if (s.selectedIds.size === 0 && !page) page = "select";
  if (page && document.getElementById(`page-${page}`)) navigate(page);
  else navigate("swipe");
})();
