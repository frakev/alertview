/* ── Utilities ── */
function esc(s) {
  if (s == null) return '';
  const d = document.createElement('div');
  d.textContent = String(s);
  return d.innerHTML;
}

function relTime(iso) {
  try {
    const d = Math.max(0, Date.now() - new Date(iso).getTime());
    const m = Math.floor(d / 60000);
    const h = Math.floor(m / 60);
    const days = Math.floor(h / 24);
    if (days > 0) return days + 'j ' + (h % 24) + 'h';
    if (h  > 0) return h + 'h ' + (m % 60) + 'm';
    if (m  > 0) return m + 'm';
    return 'à l\'instant';
  } catch { return '?' }
}

function absTime(iso) {
  try { return new Date(iso).toLocaleString('fr-FR'); }
  catch { return iso; }
}

/* ── Theme ── */
const SUN  = '<circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>';
const MOON = '<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>';

function applyTheme(t) {
  document.documentElement.setAttribute('data-theme', t);
  localStorage.setItem('av-theme', t);
  document.getElementById('theme-ico').innerHTML    = t === 'dark' ? MOON : SUN;
  document.getElementById('tv-theme-ico').innerHTML = t === 'dark' ? MOON : SUN;
}

applyTheme(localStorage.getItem('av-theme') || 'dark');
document.getElementById('theme-btn').addEventListener('click', () => {
  applyTheme(document.documentElement.getAttribute('data-theme') === 'dark' ? 'light' : 'dark');
  pushUrl();
});
document.getElementById('tv-theme-btn').addEventListener('click', () => {
  applyTheme(document.documentElement.getAttribute('data-theme') === 'dark' ? 'light' : 'dark');
  pushUrl();
});

/* ── Notifications ── */
const NotifBtn = document.getElementById('notif-btn');

function updateNotifBtn() {
  if (!('Notification' in window)) { NotifBtn.style.display = 'none'; return; }
  NotifBtn.className = 'icon-btn';
  if (Notification.permission === 'granted') NotifBtn.classList.add('notif-granted');
  if (Notification.permission === 'denied')  NotifBtn.classList.add('notif-denied');
  NotifBtn.title = { granted: 'Notifications activées', denied: 'Notifications bloquées', default: 'Activer les notifications' }[Notification.permission] || 'Notifications';
}
NotifBtn.addEventListener('click', async () => {
  if (!('Notification' in window)) return;
  if (Notification.permission === 'denied') { alert('Notifications bloquées — modifiez les paramètres du site.'); return; }
  await Notification.requestPermission();
  updateNotifBtn();
});
updateNotifBtn();

function sendNotif(newAlerts) {
  if (Notification?.permission !== 'granted' || !newAlerts.length) return;
  const bySev = s => newAlerts.filter(a => a.severity === s).length;
  const icon = bySev('critical') ? '🔴' : bySev('high') ? '🟠' : '🟡';
  const n = new Notification(
    `${icon} ${newAlerts.length} nouvelle${newAlerts.length > 1 ? 's' : ''} alerte${newAlerts.length > 1 ? 's' : ''}`,
    { body: newAlerts.slice(0, 6).map(a => `[${a.severity.toUpperCase()}] ${a.name}`).join('\n') + (newAlerts.length > 6 ? `\n… et ${newAlerts.length - 6} de plus` : '') }
  );
  n.onclick = () => { window.focus(); n.close(); };
  setTimeout(() => n.close(), 9000);
}

/* ── App state (filters persistés dans localStorage) ── */
const App = {
  data:           null,
  knownFps:       null,
  freshFps:       new Set(),
  searchQ:        '',
  sevFilter:      localStorage.getItem('av-sev-filter') || 'all',
  srcFilter:      localStorage.getItem('av-src-filter') || 'all',
  showSilenced:   localStorage.getItem('av-show-silenced') === 'true',
  refreshTimer:   null,
  countdownTimer: null,
  countdown:      0,
  loading:        false,
};

/* ── Search ── */
const SearchInput = document.getElementById('search');
const SearchClear = document.getElementById('search-clear');
SearchInput.addEventListener('input', e => {
  App.searchQ = e.target.value;
  SearchClear.style.display = App.searchQ ? 'block' : 'none';
  renderAlerts();
  pushUrl();
});
SearchClear.addEventListener('click', () => {
  SearchInput.value = App.searchQ = '';
  SearchClear.style.display = 'none';
  renderAlerts();
});

/* ── Silence toggle ── */
function updateSilenceBtn() {
  [document.getElementById('silence-btn'), document.getElementById('tv-silence-btn')].forEach(btn => {
    if (!btn) return;
    btn.textContent = App.showSilenced ? 'Masquer silencées' : 'Afficher silencées';
    btn.classList.toggle('active', App.showSilenced);
  });
}
function toggleSilenced() {
  App.showSilenced = !App.showSilenced;
  localStorage.setItem('av-show-silenced', App.showSilenced);
  updateSilenceBtn();
  renderAlerts();
  pushUrl();
}
document.getElementById('silence-btn').addEventListener('click', toggleSilenced);
document.getElementById('tv-silence-btn').addEventListener('click', toggleSilenced);

/* ── Source filter (normal mode) ── */
const SourceSel = document.getElementById('source-sel');
SourceSel.addEventListener('change', e => {
  App.srcFilter = e.target.value;
  localStorage.setItem('av-src-filter', e.target.value);
  syncTvSrcSel();
  renderAlerts();
  pushUrl();
});

/* ── Refresh ── */
document.getElementById('refresh-btn').addEventListener('click', () => fetchAlerts());

/* ── Fetch ── */
async function fetchAlerts() {
  if (App.loading) return;
  App.loading = true;
  document.getElementById('spinner').style.display = 'block';
  clearTimeout(App.refreshTimer);
  clearInterval(App.countdownTimer);

  try {
    const resp = await fetch('/api/alerts');
    if (!resp.ok) throw new Error('HTTP ' + resp.status);
    const data = await resp.json();

    const curFps = new Set(data.alerts.map(a => a.fingerprint));
    App.freshFps = new Set();
    if (App.knownFps !== null) {
      const newAlerts = data.alerts.filter(a => !App.knownFps.has(a.fingerprint));
      if (newAlerts.length) { sendNotif(newAlerts); newAlerts.forEach(a => App.freshFps.add(a.fingerprint)); }
    }
    App.knownFps = curFps;
    App.data = data;

    render();
    syncSourceSel();

    document.getElementById('last-refresh').textContent = new Date().toLocaleTimeString('fr-FR');

    App.countdown = data.refresh_interval;
    document.getElementById('countdown').textContent = App.countdown;
    document.getElementById('tv-cd').textContent = App.countdown;

    App.countdownTimer = setInterval(() => {
      App.countdown = Math.max(0, App.countdown - 1);
      document.getElementById('countdown').textContent = App.countdown;
      document.getElementById('tv-cd').textContent = App.countdown;
    }, 1000);

    App.refreshTimer = setTimeout(fetchAlerts, data.refresh_interval * 1000);
  } catch (err) {
    console.error('AlertView:', err);
    App.refreshTimer = setTimeout(fetchAlerts, 15000);
  } finally {
    App.loading = false;
    document.getElementById('spinner').style.display = 'none';
  }
}

function syncSourceSel() {
  [SourceSel, document.getElementById('tv-src-sel')].forEach(sel => {
    const cur = App.srcFilter;
    sel.innerHTML = `<option value="all">${sel === SourceSel ? 'Toutes les sources' : 'Toutes'}</option>`;
    (App.data?.sources ?? []).forEach(s => {
      const o = document.createElement('option');
      o.value = s.name;
      o.textContent = sel === SourceSel ? `${s.name} (${s.alert_count})` : s.name;
      sel.appendChild(o);
    });
    if ([...sel.options].some(o => o.value === cur)) sel.value = cur;
  });
}

function syncTvSrcSel() {
  const sel = document.getElementById('tv-src-sel');
  if ([...sel.options].some(o => o.value === App.srcFilter)) sel.value = App.srcFilter;
  if ([...SourceSel.options].some(o => o.value === App.srcFilter)) SourceSel.value = App.srcFilter;
}

/* ── Filters ── */
function filteredAlerts() {
  return (App.data?.alerts ?? []).filter(a => {
    if (!App.showSilenced && a.status === 'silenced') return false;
    if (App.sevFilter !== 'all' && a.severity !== App.sevFilter) return false;
    if (App.srcFilter !== 'all' && a.source !== App.srcFilter) return false;
    if (App.searchQ) {
      const q = App.searchQ.toLowerCase();
      const hay = [a.name, a.severity, a.source, a.status,
        ...Object.values(a.labels ?? {}), ...Object.values(a.annotations ?? {})].join('\n').toLowerCase();
      if (!hay.includes(q)) return false;
    }
    return true;
  });
}

function toggleSev(s) {
  App.sevFilter = App.sevFilter === s ? 'all' : s;
  localStorage.setItem('av-sev-filter', App.sevFilter);
  renderStats();
  renderAlerts();
  TV.renderChips();
  pushUrl();
}

/* ── Render ── */
function render() { renderStats(); renderSources(); renderAlerts(); TV.renderChips(); TV.renderDots(); updateSilenceBtn(); }

function renderStats() {
  const counts = {};
  (App.data?.alerts ?? []).forEach(a => { const s = a.severity || 'none'; counts[s] = (counts[s] || 0) + 1; });
  const order = ['critical','high','warning','info','none'];
  document.getElementById('stats-bar').innerHTML = order.filter(s => counts[s])
    .map(s => `<span class="stat-chip sev-${s}${App.sevFilter === s ? ' active' : ''}" onclick="toggleSev('${s}')">${counts[s]}&thinsp;${s}</span>`)
    .join('');
}

function renderSources() {
  document.getElementById('sources-bar').innerHTML = (App.data?.sources ?? []).map((s, i) => `
    ${i > 0 ? '<span class="src-sep">·</span>' : ''}
    <span class="src-item">
      <span class="src-dot ${s.status}"></span>
      ${esc(s.name)}
      ${s.status === 'ok' ? '· ' + s.alert_count + ' alerte' + (s.alert_count !== 1 ? 's' : '') : `<span class="src-err-label" title="${esc(s.error)}">⚠ erreur</span>`}
    </span>`).join('');
}

function renderAlerts() {
  const filtered = filteredAlerts();
  const total    = App.data?.alerts.length ?? 0;
  const listEl   = document.getElementById('alert-list');

  document.getElementById('alert-count').textContent = filtered.length < total
    ? filtered.length + ' / ' + total + ' alertes'
    : total + ' alerte' + (total !== 1 ? 's' : '');

  if (!filtered.length) {
    listEl.innerHTML = `<div class="empty-state">
      <div class="empty-state-icon">${App.searchQ || App.sevFilter !== 'all' || App.srcFilter !== 'all' ? '🔍' : '✅'}</div>
      <div>${App.searchQ ? 'Aucun résultat pour &laquo;&nbsp;' + esc(App.searchQ) + '&nbsp;&raquo;' : 'Aucune alerte active'}</div>
    </div>`;
    return;
  }

  listEl.innerHTML = filtered.map(cardHtml).join('');

  if (App.freshFps.size) {
    listEl.querySelectorAll('.alert-card').forEach(el => {
      if (App.freshFps.has(el.dataset.fp)) el.classList.add('new');
    });
  }
}

function genLinkHtml(url) {
  if (!url) return '';
  return `<a href="${esc(url)}" target="_blank" rel="noopener noreferrer" class="gen-link" title="Ouvrir dans Prometheus/Grafana">
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
      <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
      <polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/>
    </svg></a>`;
}

function cardHtml(a) {
  if (TV.active) return cardHtmlTV(a);

  const sev    = a.severity || 'none';
  const labels = (App.data?.display_labels ?? [])
    .filter(l => a.labels?.[l] !== undefined && l !== 'alertname' && l !== 'severity')
    .map(l => `<span class="lbl">${esc(l)}=<b>${esc(a.labels[l])}</b></span>`).join('');

  const summary  = a.annotations?.summary || '';
  const desc     = a.annotations?.description || '';
  const showDesc = desc && desc !== summary;

  return `
    <div class="alert-card sev-${sev}" data-fp="${esc(a.fingerprint)}">
      <div class="card-top">
        <div class="card-title">
          <span class="sev-dot sev-${sev}"></span>
          <span class="alert-name">${esc(a.name)}</span>
          <span class="sev-badge sev-${sev}">${sev}</span>
          <span class="status-badge status-${a.status}">${a.status}</span>
        </div>
        <div class="card-meta">
          <span class="src-chip">${esc(a.source)}</span>
          <span class="time-ago" title="${esc(absTime(a.starts_at))}">depuis&nbsp;${relTime(a.starts_at)}</span>
          ${genLinkHtml(a.link_url)}
        </div>
      </div>
      ${summary  ? `<div class="card-summary">${esc(summary)}</div>` : ''}
      ${showDesc ? `<div class="card-desc">${esc(desc)}</div>` : ''}
      ${labels   ? `<div class="label-chips">${labels}</div>` : ''}
    </div>`;
}

function cardHtmlTV(a) {
  const sev     = a.severity || 'none';
  const summary = a.annotations?.summary || '';
  return `
    <div class="alert-card alert-row sev-${sev}" data-fp="${esc(a.fingerprint)}">
      <span class="sev-dot sev-${sev}"></span>
      <span class="alert-name">${esc(a.name)}</span>
      <span class="sev-badge sev-${sev}">${sev}</span>
      <span class="status-badge status-${a.status}">${a.status}</span>
      <span class="row-summary">${esc(summary)}</span>
      <span class="src-chip">${esc(a.source)}</span>
      <span class="time-ago" title="${esc(absTime(a.starts_at))}">depuis&nbsp;${relTime(a.starts_at)}</span>
      ${genLinkHtml(a.link_url)}
    </div>`;
}

/* ── TV Mode ── */
const TV = {
  active:     false,
  panelOpen:  false,
  clockTimer: null,
  hideTimer:  null,

  init() {
    this.active = localStorage.getItem('av-tv') === 'true';
    if (this.active) this._apply();

    document.getElementById('tv-btn').addEventListener('click',      () => this.toggle());
    document.getElementById('tv-settings-btn').addEventListener('click', e => { e.stopPropagation(); this.togglePanel(); });
    document.getElementById('tv-exit-btn').addEventListener('click', () => this.toggle());

    document.getElementById('tv-src-sel').addEventListener('change', e => {
      App.srcFilter = e.target.value;
      localStorage.setItem('av-src-filter', e.target.value);
      syncTvSrcSel();
      renderAlerts();
      pushUrl();
    });

    // Afficher la barre au moindre mouvement
    document.addEventListener('mousemove', () => this.showBar());
    document.addEventListener('click',     () => this.showBar());

    // Fermer le panneau en cliquant ailleurs
    document.addEventListener('click', e => {
      if (this.panelOpen && !document.getElementById('tv-panel').contains(e.target) && e.target.id !== 'tv-settings-btn') {
        this.closePanel();
      }
    });

    // Raccourcis clavier
    document.addEventListener('keydown', e => {
      if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT') return;
      if (e.key === 'Escape') { if (this.panelOpen) this.closePanel(); else if (this.active) this.toggle(); }
      if ((e.key === 't' || e.key === 'T') && !e.ctrlKey && !e.metaKey) this.toggle();
    });
  },

  toggle() {
    this.active = !this.active;
    localStorage.setItem('av-tv', this.active);
    this._apply();
  },

  _apply() {
    document.documentElement.setAttribute('data-tv', this.active);
    document.getElementById('tv-btn').classList.toggle('active', this.active);
    if (this.active) {
      this.startClock();
      this.showBar();
      this.renderChips();
      this.renderDots();
      syncTvSrcSel();
    } else {
      this.stopClock();
      this.closePanel();
      clearTimeout(this.hideTimer);
    }
    renderAlerts();
  },

  startClock() {
    this.updateClock();
    this.clockTimer = setInterval(() => this.updateClock(), 1000);
  },
  stopClock() { clearInterval(this.clockTimer); },
  updateClock() {
    document.getElementById('tv-clock').textContent =
      new Date().toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  },

  showBar() {
    if (!this.active) return;
    const bar = document.getElementById('tv-bar');
    bar.classList.add('visible');
    clearTimeout(this.hideTimer);
    if (!this.panelOpen) {
      this.hideTimer = setTimeout(() => bar.classList.remove('visible'), 4000);
    }
  },

  togglePanel() {
    this.panelOpen ? this.closePanel() : this.openPanel();
  },
  openPanel() {
    this.panelOpen = true;
    document.getElementById('tv-panel').classList.add('open');
    clearTimeout(this.hideTimer);
    document.getElementById('tv-bar').classList.add('visible');
  },
  closePanel() {
    this.panelOpen = false;
    document.getElementById('tv-panel').classList.remove('open');
    this.showBar();
  },

  renderChips() {
    const counts = {};
    (App.data?.alerts ?? []).forEach(a => { const s = a.severity || 'none'; counts[s] = (counts[s] || 0) + 1; });
    const order = ['critical','high','warning','info','none'];
    const all = `<span class="stat-chip${App.sevFilter === 'all' ? ' active' : ''}" style="font-size:10px;padding:1px 7px" onclick="toggleSev('all')">tout</span>`;
    document.getElementById('tv-sev-chips').innerHTML = all + order.filter(s => counts[s])
      .map(s => `<span class="stat-chip sev-${s}${App.sevFilter === s ? ' active' : ''}" style="font-size:10px;padding:1px 7px" onclick="toggleSev('${s}')">${counts[s]}&thinsp;${s}</span>`)
      .join('');
  },

  renderDots() {
    document.getElementById('tv-dots').innerHTML = (App.data?.sources ?? [])
      .map(s => `<span class="src-dot ${s.status}" title="${esc(s.name)}${s.error ? ': ' + esc(s.error) : ''}"></span>`)
      .join('');
  },
};

TV.init();

/* ── URL state sync ── */
function pushUrl() {
  const p = new URLSearchParams();
  const theme = document.documentElement.getAttribute('data-theme');
  if (theme !== 'dark')        p.set('theme', theme);
  if (App.sevFilter !== 'all') p.set('sev',      App.sevFilter);
  if (App.srcFilter !== 'all') p.set('src',      App.srcFilter);
  if (App.searchQ)             p.set('q',        App.searchQ);
  if (App.showSilenced)        p.set('silenced', '1');
  const qs = p.toString();
  history.replaceState(null, '', qs ? '?' + qs : location.pathname);
}

function initFromUrl() {
  const p = new URLSearchParams(location.search);
  if (p.has('theme')) applyTheme(p.get('theme'));
  if (p.has('sev'))   { App.sevFilter = p.get('sev');   localStorage.setItem('av-sev-filter', App.sevFilter); }
  if (p.has('src'))   { App.srcFilter = p.get('src');   localStorage.setItem('av-src-filter', App.srcFilter); }
  if (p.has('q'))        {
    App.searchQ = p.get('q');
    SearchInput.value = App.searchQ;
    SearchClear.style.display = App.searchQ ? 'block' : 'none';
  }
  if (p.has('silenced')) { App.showSilenced = p.get('silenced') === '1'; localStorage.setItem('av-show-silenced', App.showSilenced); }
}

/* ── Boot ── */
initFromUrl();
updateSilenceBtn();
fetchAlerts();
