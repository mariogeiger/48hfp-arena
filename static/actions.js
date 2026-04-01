import { USER_ID, api, store } from "./store.js";

// ==================== ACTIONS ====================

let saveTimeout = null;

export function navigate(page) {
  store.set((s) => ({ ...s, page }));
  document.body.classList.toggle("swipe-active", page === "swipe");
  location.hash = page;
  const loaders = { swipe: loadPair, board: loadBoard, more: loadMore };
  if (loaders[page]) loaders[page]();
}

export function toggleFilm(id) {
  store.set((s) => {
    const next = new Set(s.selectedIds);
    next.has(id) ? next.delete(id) : next.add(id);
    return { ...s, selectedIds: next };
  });
  debounceSave();
}

export function selectAll() {
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

export function deselectAll() {
  store.set((s) => ({ ...s, selectedIds: new Set() }));
  debounceSave();
}

export function setSearch(query) {
  store.set((s) => ({ ...s, searchQuery: query }));
}

function debounceSave() {
  clearTimeout(saveTimeout);
  saveTimeout = setTimeout(saveSelection, 500);
}

export async function saveSelection() {
  const ids = [...store.get().selectedIds];
  localStorage.setItem("filmrank_selected", JSON.stringify(ids));
  await api("/api/selection", { user_id: USER_ID, film_ids: ids });
}

export function setFocusFilm(id) {
  store.set((s) => ({ ...s, focusFilmId: id }));
  loadPair();
}

export async function loadPair() {
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

export async function castVote(winnerId, loserId) {
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

export async function undoVote() {
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

export async function deselectAndSkip(filmId) {
  store.set((s) => {
    const next = new Set(s.selectedIds);
    next.delete(filmId);
    return { ...s, selectedIds: next };
  });
  await saveSelection();
  loadPair();
}

export async function loadBoard() {
  const data = await api("/api/leaderboard");
  store.set((s) => ({ ...s, board: data }));
}

export async function loadMore() {
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

export async function matrixAction(endpoint, w, l) {
  await api(endpoint, { user_id: USER_ID, winner_id: w, loser_id: l });
  const [userMatrix, globalMatrix] = await Promise.all([
    api(`/api/user-matrix?user_id=${USER_ID}&_=${Date.now()}`),
    api(`/api/global-matrix?_=${Date.now()}`),
  ]);
  store.set((s) => ({ ...s, userMatrix, globalMatrix }));
}

export async function resetVotes() {
  if (!confirm("This will permanently delete all your votes. Are you sure?"))
    return;
  await api("/api/reset-votes", { user_id: USER_ID });
  loadMore();
  loadBoard();
  loadPair();
}

export function submitSuggestion() {
  const filmSelect = document.getElementById("suggest-film");
  const title = document.getElementById("suggest-title").value.trim();
  const team = document.getElementById("suggest-team").value.trim();
  const city = document.getElementById("suggest-city").value.trim();
  const poster = document.getElementById("suggest-poster").value.trim();
  const video = document.getElementById("suggest-video").value.trim();

  if (!title && !team && !video) return;

  const isNew = !filmSelect.value;
  const filmName = isNew
    ? title || team
    : filmSelect.options[filmSelect.selectedIndex]?.text;
  const issueTitle = isNew
    ? `New film: ${filmName}`
    : `Correction: ${filmName}`;
  const lines = [];
  if (title) lines.push(`- **Title:** ${title}`);
  if (team) lines.push(`- **Team:** ${team}`);
  if (city) lines.push(`- **City:** ${city}`);
  if (poster) lines.push(`- **Poster URL:** ${poster}`);
  if (video) lines.push(`- **Video URL:** ${video}`);
  const heading = isNew ? "## New film suggestion" : "## Suggested correction";
  const body = `${heading}\n\n${lines.join("\n")}\n`;

  const url = `https://github.com/mariogeiger/48hfp-arena/issues/new?title=${encodeURIComponent(issueTitle)}&body=${encodeURIComponent(body)}`;
  window.open(url, "_blank");
}

export function addToast(html) {
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
