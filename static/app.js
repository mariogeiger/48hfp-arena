const USER_ID = localStorage.getItem('filmrank_uid') || (() => {
  const id = crypto.randomUUID();
  localStorage.setItem('filmrank_uid', id);
  return id;
})();

let allFilms = [];
let selectedIds = new Set(JSON.parse(localStorage.getItem('filmrank_selected') || '[]'));
let currentPair = null;
let voteCount = 0;
let voteHistory = []; // [{winnerId, loserId}, ...]

// -- Helpers --
function esc(s) {
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}

function posterImg(url) {
  if (!url) return `<div class="poster-ph">&#127902;</div>`;
  return `<img class="poster" src="${esc(url)}" loading="lazy" onerror="this.outerHTML='<div class=\\'poster-ph\\'>&#127902;</div>'">`;
}

function filmMeta(f) {
  return `${esc(f.team)}${f.city ? ' &middot; ' + esc(f.city) : ''}`;
}

function barRow(label, value, fraction) {
  return `<div class="stats-bar-row">
    <div class="stats-bar-label" title="${esc(label)}">${esc(label)}</div>
    <div class="stats-bar-track">
      <div class="stats-bar-fill" style="width: ${(fraction * 100).toFixed(1)}%"></div>
    </div>
    <div class="stats-bar-value">${value}</div>
  </div>`;
}

// -- Init --
async function init() {
  const [filmsRes, boardRes] = await Promise.all([
    fetch('/api/films'),
    fetch('/api/leaderboard'),
  ]);
  allFilms = await filmsRes.json();
  const board = await boardRes.json();
  const rankById = new Map(board.map((item, i) => [item.film_id, i]));
  allFilms.sort((a, b) => (rankById.get(a.id) ?? Infinity) - (rankById.get(b.id) ?? Infinity));
  renderFilmList(allFilms);
  updateSelectionStatus();
  if (selectedIds.size >= 2) saveSelection();
}

// -- Page Navigation --
function showPage(page) {
  document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
  document.querySelectorAll('.nav button').forEach(b => b.classList.remove('active'));
  document.getElementById(`page-${page}`).classList.add('active');
  document.getElementById(`nav-${page}`).classList.add('active');
  location.hash = page;

  const loaders = { swipe: loadPair, board: loadLeaderboard, stats: loadStats };
  if (loaders[page]) loaders[page]();
}

// -- PAGE 1: Film Selection --
function renderFilmList(films) {
  document.getElementById('film-list').innerHTML = films.map(f => `
    <div class="film-item ${selectedIds.has(f.id) ? 'selected' : ''}"
         data-id="${f.id}" onclick="toggleFilm(${f.id}, this)">
      <div class="film-check"></div>
      ${posterImg(f.poster_url)}
      <div class="film-info">
        <div class="film-title">${esc(f.title)}</div>
        <div class="film-meta">${filmMeta(f)}</div>
      </div>
    </div>
  `).join('');
}

let saveTimeout = null;

function toggleFilm(id, el) {
  if (selectedIds.has(id)) { selectedIds.delete(id); el.classList.remove('selected'); }
  else { selectedIds.add(id); el.classList.add('selected'); }
  updateSelectionStatus();
  debounceSaveSelection();
}

function filterFilms(q) {
  q = q.toLowerCase();
  renderFilmList(allFilms.filter(f =>
    f.title.toLowerCase().includes(q) ||
    f.team.toLowerCase().includes(q) ||
    f.city.toLowerCase().includes(q)
  ));
}

function selectAll() {
  document.querySelectorAll('.film-item').forEach(el => {
    selectedIds.add(parseInt(el.dataset.id));
    el.classList.add('selected');
  });
  updateSelectionStatus();
  debounceSaveSelection();
}

function deselectAll() {
  selectedIds.clear();
  document.querySelectorAll('.film-item').forEach(el => el.classList.remove('selected'));
  updateSelectionStatus();
  debounceSaveSelection();
}

function updateSelectionStatus() {
  const el = document.getElementById('selection-status');
  const n = selectedIds.size;
  el.textContent = n < 2 ? 'Select at least 2 films to compare' : `${n} films selected`;
  el.classList.toggle('has-enough', n >= 2);
}

function debounceSaveSelection() {
  clearTimeout(saveTimeout);
  saveTimeout = setTimeout(saveSelection, 500);
}

async function saveSelection() {
  localStorage.setItem('filmrank_selected', JSON.stringify([...selectedIds]));
  await fetch('/api/selection', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ user_id: USER_ID, film_ids: [...selectedIds] }),
  });
}

// -- PAGE 2: Swipe Comparison --
async function loadPair() {
  const container = document.getElementById('swipe-container');
  const res = await fetch(`/api/pair?user_id=${USER_ID}`);
  const data = await res.json();

  voteCount = data.votes || 0;

  if (data.done) {
    const notEnough = selectedIds.size < 2;
    container.innerHTML = `
      <div class="swipe-done">
        <h2>${notEnough ? 'Select films first' : 'All done!'}</h2>
        <p>${notEnough
          ? "Go back and pick at least 2 films you've watched."
          : `You've compared all possible pairs. (${voteCount} votes cast)`}</p>
        <button onclick="showPage('${notEnough ? 'select' : 'board'}')">${notEnough ? 'Select Films' : 'View Leaderboard'}</button>
      </div>`;
    return;
  }

  currentPair = data;
  renderPair(data.a, data.b);
}

function renderPair(a, b) {
  currentPair = { a, b };
  document.getElementById('swipe-container').innerHTML = `
    <div>
      <div class="swipe-card" id="swipe-card">
        <div class="swipe-overlay left-o" id="overlay-left">${esc(a.title)}</div>
        <div class="swipe-overlay right-o" id="overlay-right">${esc(b.title)}</div>
        <div class="vs-label">VS</div>
        <div class="swipe-pair">
          <div class="swipe-film">
            ${posterImg(a.poster_url)}
            <div class="title">${esc(a.title)}</div>
            <div class="meta">${filmMeta(a)}</div>
          </div>
          <div class="swipe-film">
            ${posterImg(b.poster_url)}
            <div class="title">${esc(b.title)}</div>
            <div class="meta">${filmMeta(b)}</div>
          </div>
        </div>
        <div class="swipe-buttons">
          <button class="swipe-arrow-btn pick-a" onclick="castVote(currentPair.a.id, currentPair.b.id)" title="Pick left">&larr;</button>
          <button class="swipe-arrow-btn pick-b" onclick="castVote(currentPair.b.id, currentPair.a.id)" title="Pick right">&rarr;</button>
        </div>
      </div>
      <div class="swipe-progress">${voteCount} comparisons made</div>
      <div class="swipe-bottom-actions">
        <button class="undo-btn" onclick="undoVote()" ${voteHistory.length > 0 ? '' : 'disabled'}>Undo</button>
        <button class="skip-btn" onclick="loadPair()">Skip</button>
      </div>
    </div>`;
  setupSwipe();
}

let swipeCleanup = null;

function setupSwipe() {
  if (swipeCleanup) { swipeCleanup(); swipeCleanup = null; }

  const card = document.getElementById('swipe-card');
  if (!card) return;

  let startX = 0, currentX = 0, dragging = false;

  function onStart(x) { startX = x; currentX = 0; dragging = true; card.style.transition = 'none'; }
  function onMove(x) {
    if (!dragging) return;
    currentX = x - startX;
    card.style.transform = `translateX(${currentX}px) rotate(${currentX * 0.05}deg)`;
    const ol = document.getElementById('overlay-left');
    const or2 = document.getElementById('overlay-right');
    if (ol) ol.style.opacity = currentX < -30 ? Math.min(1, (-currentX - 30) / 80) : 0;
    if (or2) or2.style.opacity = currentX > 30 ? Math.min(1, (currentX - 30) / 80) : 0;
  }
  function onEnd() {
    if (!dragging) return;
    dragging = false;
    card.style.transition = 'transform 0.3s ease, opacity 0.3s ease';

    if (Math.abs(currentX) > 80) {
      const goRight = currentX > 0;
      card.style.transform = `translateX(${goRight ? 400 : -400}px) rotate(${goRight ? 20 : -20}deg)`;
      card.style.opacity = '0';
      setTimeout(() => {
        if (goRight) castVote(currentPair.b.id, currentPair.a.id);
        else castVote(currentPair.a.id, currentPair.b.id);
      }, 200);
    } else {
      card.style.transform = '';
      document.getElementById('overlay-left').style.opacity = 0;
      document.getElementById('overlay-right').style.opacity = 0;
    }
  }

  const handlers = [
    [card, 'mousedown', e => onStart(e.clientX)],
    [window, 'mousemove', e => onMove(e.clientX)],
    [window, 'mouseup', onEnd],
    [card, 'touchstart', e => onStart(e.touches[0].clientX), { passive: true }],
    [window, 'touchmove', e => onMove(e.touches[0].clientX), { passive: true }],
    [window, 'touchend', onEnd],
    [document, 'keydown', e => {
      if (e.key === 'ArrowLeft') castVote(currentPair.a.id, currentPair.b.id);
      else if (e.key === 'ArrowRight') castVote(currentPair.b.id, currentPair.a.id);
    }],
  ];

  handlers.forEach(([el, evt, fn, opts]) => el.addEventListener(evt, fn, opts));
  swipeCleanup = () => handlers.forEach(([el, evt, fn]) => el.removeEventListener(evt, fn));
}

async function castVote(winnerId, loserId) {
  await fetch('/api/vote', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ user_id: USER_ID, winner_id: winnerId, loser_id: loserId }),
  });
  voteHistory.push({ winnerId, loserId });
  loadPair();
}

async function undoVote() {
  const last = voteHistory.pop();
  if (!last) return;
  await fetch('/api/undo', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ user_id: USER_ID, winner_id: last.winnerId, loser_id: last.loserId }),
  });
  voteCount = Math.max(0, voteCount - 1);
  const filmA = allFilms.find(f => f.id === last.winnerId);
  const filmB = allFilms.find(f => f.id === last.loserId);
  renderPair(filmA, filmB);
}

// -- PAGE 3: Leaderboard --
async function loadLeaderboard() {
  const res = await fetch('/api/leaderboard');
  const data = await res.json();
  const container = document.getElementById('board-list');

  if (data.length === 0) {
    container.innerHTML = `
      <div class="board-empty">
        <h3>No votes yet</h3>
        <p>Start comparing films to build the leaderboard!</p>
      </div>`;
    return;
  }

  container.innerHTML = data.map((item, i) => `
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
  `).join('');
}

// -- PAGE 4: Stats --
async function loadStats() {
  const res = await fetch('/api/stats');
  const s = await res.json();
  const container = document.getElementById('stats-content');

  const maxSelected = s.most_selected_films.length > 0 ? s.most_selected_films[0].count : 1;

  container.innerHTML = `
    <div class="stats-grid">
      ${[
        [s.total_votes, 'Total Votes'],
        [s.active_users, 'Active Voters'],
        [s.total_users, 'Total Users'],
        [s.avg_votes_per_user, 'Avg Votes / User'],
        [s.films_with_votes, 'Films Voted On'],
        [s.total_films, 'Total Films'],
      ].map(([v, l]) => `
        <div class="stat-card">
          <div class="stat-value">${v}</div>
          <div class="stat-label">${l}</div>
        </div>`).join('')}
    </div>
    <div class="stats-section">
      <h3>Most Selected Films</h3>
      ${s.most_selected_films.map(f => barRow(f.title, f.count, f.count / maxSelected)).join('')}
    </div>
    ${s.vote_distribution.length > 0 ? `
    <div class="stats-section">
      <h3>Votes Per User Distribution</h3>
      ${s.vote_distribution.map(d =>
        barRow(`${d.votes} vote${d.votes !== 1 ? 's' : ''}`, d.users, d.users / s.active_users)
      ).join('')}
    </div>` : ''}
  `;
}

// -- Vote Notifications (SSE) --
function initVoteStream() {
  const es = new EventSource('/api/vote/stream');
  es.onmessage = (e) => {
    const data = JSON.parse(e.data);
    if (data.user_id === USER_ID) return;
    const container = document.getElementById('toast-container');
    const toast = document.createElement('div');
    toast.className = 'vote-toast';
    toast.innerHTML = `Someone voted: <strong>${esc(data.winner_title)}</strong> over ${esc(data.loser_title)}`;
    container.appendChild(toast);
    setTimeout(() => toast.remove(), 4000);
  };
  es.onerror = () => { es.close(); setTimeout(initVoteStream, 5000); };
}

initVoteStream();
init().then(() => {
  const page = location.hash.slice(1);
  if (page && document.getElementById(`page-${page}`)) showPage(page);
});
