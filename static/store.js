// ==================== CONSTANTS ====================

export const USER_ID =
  localStorage.getItem("filmrank_uid") ||
  (() => {
    const id = crypto.randomUUID();
    localStorage.setItem("filmrank_uid", id);
    return id;
  })();

// ==================== UTILITIES ====================

export function esc(s) {
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}

export function api(path, body) {
  const opts = body
    ? {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      }
    : {};
  return fetch(path, opts).then((r) => {
    if (r.status === 403) {
      document.getElementById("banned-overlay").classList.remove("hidden");
    }
    return r.json();
  });
}

export const brokenPosters = new Set();

export function posterHtml(url) {
  if (!url || brokenPosters.has(url))
    return `<div class="poster-ph">&#127916;</div>`;
  return `<img class="poster" src="${esc(url)}">`;
}

export function metaHtml(f) {
  return `${esc(f.team)}${f.city ? " &middot; " + esc(f.city) : ""}`;
}

export function shortTitle(t) {
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

export const store = createStore({
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
