/* -- Utilities -- */
/* localStorage throws, it does not just return null: private browsing and
   "block site data" both make every access raise. Reading it unguarded at
   startup took the whole script down and left a blank page. */
function lsGet(key) {
  try { return localStorage.getItem(key); } catch { return null; }
}

function lsSet(key, value) {
  try { localStorage.setItem(key, value); } catch { /* storage unavailable */ }
}

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
    if (days > 0) return days + 'd ' + (h % 24) + 'h';
    if (h  > 0) return h + 'h ' + (m % 60) + 'm';
    if (m  > 0) return m + 'm';
    return 'just now';
  } catch { return '?' }
}

function absTime(iso) {
  try {
    const date = new Date(iso);
    if (AppConfig.timezone === 'local' || AppConfig.timezone === 'UTC') {
      return date.toLocaleString('en-US', { timeZone: AppConfig.timezone === 'UTC' ? 'UTC' : undefined });
    }
    return date.toLocaleString('en-US', { timeZone: AppConfig.timezone });
  } catch { return iso; }
}

/* -- Theme --
   The stored preference is "auto" | "light" | "dark"; `data-theme` on <html>
   always holds the *resolved* value (light or dark) so the CSS never has to
   know about "auto". In auto the OS preference is followed live. */
const THEME_COLORS = { dark: '#0d1117', light: '#f6f8fa' };
const AUTO = '<circle cx="12" cy="12" r="9"/><path d="M12 3v18a9 9 0 0 0 0-18z" fill="currentColor" stroke="none"/>';
const SUN  = '<circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>';
const MOON = '<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>';

// Sounds context
const AudioContext = (() => {
  let ctx = null;
  return {
    get: () => {
      if (!ctx) {
        try {
          ctx = new (window.AudioContext || window.webkitAudioContext)();
        } catch (e) {
          console.warn('Web Audio API not available:', e);
          return null;
        }
      }
      return ctx;
    },
    playBeep: (frequency = 440, duration = 200) => {
      const ctx = AudioContext.get();
      if (!ctx) return;
      
      const oscillator = ctx.createOscillator();
      const gainNode = ctx.createGain();
      
      oscillator.connect(gainNode);
      gainNode.connect(ctx.destination);
      
      oscillator.frequency.value = frequency;
      oscillator.type = 'sine';
      
      gainNode.gain.setValueAtTime(0.1, ctx.currentTime);
      gainNode.gain.exponentialRampToValueAtTime(0.01, ctx.currentTime + duration / 1000);
      
      oscillator.start(ctx.currentTime);
      oscillator.stop(ctx.currentTime + duration / 1000);
    }
  };
})();

// Severity ranking. The server sends display.severity_order from the config;
// these are only the fallback used before the first payload arrives.
const DEFAULT_SEV_ORDER = ['critical','error','high','warning','info','none'];
const SEV_ALIASES = { crit: 'critical', err: 'error', warn: 'warning', information: 'info' };

function sevOrderList() {
  const order = App.data?.severity_order;
  return order?.length ? order : DEFAULT_SEV_ORDER;
}

/* Severity comes from an alert label, i.e. from outside. It ends up in a CSS
   class, so reduce it to a safe token instead of interpolating it raw. */
function sevClass(sev) {
  const slug = String(sev || 'none').toLowerCase().replace(/[^a-z0-9_-]+/g, '-');
  return 'sev-' + (slug || 'none');
}

function canonSev(sev) {
  const s = (sev || 'none').trim().toLowerCase();
  return SEV_ALIASES[s] ?? s;
}

// Sound presets by severity
const SOUND_PRESETS = {
  critical: () => { AudioContext.playBeep(800, 300); AudioContext.playBeep(600, 300); },
  error:    () => { AudioContext.playBeep(700, 250); AudioContext.playBeep(550, 250); },
  high:     () => { AudioContext.playBeep(600, 200); AudioContext.playBeep(500, 200); },
  warning:  () => { AudioContext.playBeep(400, 150); },
  info:     () => { AudioContext.playBeep(300, 100); }
};

const THEME_PREFS = ['auto', 'light', 'dark'];

function osPrefersDark() {
  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? true;
}

function resolveTheme(pref) {
  return pref === 'auto' ? (osPrefersDark() ? 'dark' : 'light') : pref;
}

/* `persist: false` applies a theme that came from the config, without
   overwriting a choice the user made in this browser. */
function applyTheme(pref, { persist = true } = {}) {
  if (!THEME_PREFS.includes(pref)) pref = 'auto';
  App.themePref = pref;
  if (persist) {
    lsSet('av-theme', pref);
    // Without this the config theme would overwrite the click on the next poll.
    App.themeFromUser = true;
  }

  const resolved = resolveTheme(pref);
  document.documentElement.setAttribute('data-theme', resolved);

  const icon = pref === 'auto' ? AUTO : pref === 'dark' ? MOON : SUN;
  const title = pref === 'auto' ? `Theme: auto (${resolved})` : `Theme: ${pref}`;
  ['theme-ico', 'tv-theme-ico'].forEach(id => {
    const el = document.getElementById(id);
    if (el) { el.innerHTML = icon; el.parentElement.title = title; }
  });

  const meta = document.querySelector('meta[name="theme-color"]');
  if (meta) meta.setAttribute('content', THEME_COLORS[resolved]);
}

function cycleTheme() {
  applyTheme(THEME_PREFS[(THEME_PREFS.indexOf(App.themePref) + 1) % THEME_PREFS.length]);
  pushUrl();
}

// Follow the OS while the preference is "auto".
window.matchMedia?.('(prefers-color-scheme: dark)')
  .addEventListener('change', () => { if (App.themePref === 'auto') applyTheme('auto', { persist: false }); });

// Layer an extra stylesheet on top of the theme, if the config provides one.
function applyCustomTheme(cssUrl) {
  const existing = document.getElementById('custom-theme-css');
  if (existing) existing.remove();

  if (cssUrl && !THEME_PREFS.includes(cssUrl)) {
    const link = document.createElement('link');
    link.id = 'custom-theme-css';
    link.rel = 'stylesheet';
    link.href = cssUrl;
    document.head.appendChild(link);
  }
}

document.getElementById('theme-btn').addEventListener('click', cycleTheme);
document.getElementById('tv-theme-btn').addEventListener('click', cycleTheme);

/* -- knownFps persistence -- */
function loadKnownFps() {
  try {
    const raw = lsGet('av-known-fps');
    if (!raw) return null;
    const { fps, ts } = JSON.parse(raw);
    if (Date.now() - ts > 86400000) return null;
    return new Set(fps);
  } catch { return null; }
}

function saveKnownFps(fps) {
  try {
    lsSet('av-known-fps', JSON.stringify({ fps: [...fps], ts: Date.now() }));
  } catch {}
}

/* -- Notifications -- */
const NotifBtn = document.getElementById('notif-btn');

function updateNotifBtn() {
  if (!('Notification' in window)) { NotifBtn.style.display = 'none'; return; }
  NotifBtn.className = 'icon-btn';
  if (Notification.permission === 'granted') NotifBtn.classList.add('notif-granted');
  if (Notification.permission === 'denied')  NotifBtn.classList.add('notif-denied');
  NotifBtn.title = { granted: 'Notifications enabled', denied: 'Notifications blocked', default: 'Enable notifications' }[Notification.permission] || 'Notifications';
}
NotifBtn.addEventListener('click', async () => {
  if (!('Notification' in window)) return;
  if (Notification.permission === 'denied') { alert('Notifications blocked - please check site permissions.'); return; }
  await Notification.requestPermission();
  updateNotifBtn();
});
updateNotifBtn();

/* -- Server-Sent Events (SSE) for real-time notifications -- */
let sseConnected = false;
let sseRetryCount = 0;
const maxSseRetries = 5;

function connectSSE() {
  if (!('EventSource' in window)) {
    console.log('SSE not supported in this browser');
    return;
  }

  const eventSource = new EventSource('/events');
  
  eventSource.onopen = () => {
    sseConnected = true;
    sseRetryCount = 0;
    console.log('SSE connection opened');
  };

  eventSource.onerror = (err) => {
    sseConnected = false;
    console.log('SSE connection error:', err);
    eventSource.close();
    
    // Retry with exponential backoff
    if (sseRetryCount < maxSseRetries) {
      const delay = Math.pow(2, sseRetryCount) * 1000; // 2, 4, 8, 16, 32 seconds
      sseRetryCount++;
      console.log(`SSE reconnecting in ${delay}ms (attempt ${sseRetryCount}/${maxSseRetries})`);
      setTimeout(connectSSE, delay);
    } else {
      console.warn('SSE max retries reached, giving up');
    }
  };

  // An SSE event only means "something changed, refresh sooner than the next
  // poll". The refresh is debounced (a burst of new alerts is one event per
  // alert) and fetchAlerts() does its own new-alert diff, so the sound and the
  // notification are not fired from here — doing both would double them.
  eventSource.addEventListener('new_alert', () => scheduleRefresh());

  // Reload config when it changes (e.g., display_labels)
  eventSource.addEventListener('config_reloaded', () => {
    console.log('Config reloaded via SSE, refreshing alerts...');
    scheduleRefresh();
  });

  // Store reference for cleanup
  window._eventSource = eventSource;
}

// Connect to SSE when page loads
if ('EventSource' in window) {
  // Wait a bit for the page to be ready
  setTimeout(connectSSE, 1000);
}

// Global state for sounds and timezone
let AppConfig = {
  playSounds: false,
  timezone: 'local'
};

function playSoundForAlerts(newAlerts) {
  if (!AppConfig.playSounds || !newAlerts.length) return;
  
  // Play sound for the highest severity
  for (const sev of sevOrderList()) {
    if (newAlerts.some(a => canonSev(a.severity) === canonSev(sev))) {
      const preset = SOUND_PRESETS[canonSev(sev)];
      if (preset) preset();
      break; // Only play for the highest severity
    }
  }
}

function sendNotif(newAlerts) {
  if (Notification?.permission !== 'granted' || !newAlerts.length) return;
  const bySev = s => newAlerts.filter(a => a.severity === s).length;
  const icon = bySev('critical') ? '🔴' : bySev('error') ? '🟠' : bySev('high') ? '🟠' : '🟡';
  const alertWord = newAlerts.length > 1 ? 'alerts' : 'alert';
  const n = new Notification(
    `${icon} ${newAlerts.length} new ${alertWord}`,
    { body: newAlerts.slice(0, 6).map(a => `[${a.severity.toUpperCase()}] ${a.name}`).join('\n') + (newAlerts.length > 6 ? `\n... and ${newAlerts.length - 6} more` : '') }
  );
  n.onclick = () => { window.focus(); n.close(); };
  setTimeout(() => n.close(), 9000);
}

/* -- App state (filters persisted in localStorage) -- */
const App = {
  data:           null,
  themePref:      lsGet('av-theme') || 'auto',
  themeFromUser:  lsGet('av-theme') !== null,
  themeFromUrl:   false,
  tvFromUrl:      false,
  knownFps:       loadKnownFps(),
  freshFps:       new Set(),
  searchQ:        '',
  sevFilter:      lsGet('av-sev-filter') || 'all',
  srcFilter:      (() => { try { const r = lsGet('av-src-filter'); return new Set(r ? JSON.parse(r) : []); } catch { return new Set(); } })(),
  showSilenced:   lsGet('av-show-silenced') === 'true',
  refreshTimer:   null,
  countdownTimer: null,
  countdown:      0,
  loading:        false,
  openGroups:     new Set(),
  openLabels:     new Set(),
  openComments:   new Set(),
};

/* -- Search -- */
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
  pushUrl();
});

/* -- Silence toggle -- */
function updateSilenceBtn() {
  [document.getElementById('silence-btn'), document.getElementById('tv-silence-btn')].forEach(btn => {
    if (!btn) return;
    btn.textContent = App.showSilenced ? 'Hide silenced' : 'Show silenced';
    btn.classList.toggle('active', App.showSilenced);
  });
}
function toggleSilenced() {
  App.showSilenced = !App.showSilenced;
  lsSet('av-show-silenced', App.showSilenced);
  updateSilenceBtn();
  renderAlerts();
  pushUrl();
}
document.getElementById('silence-btn').addEventListener('click', toggleSilenced);
document.getElementById('tv-silence-btn').addEventListener('click', toggleSilenced);

/* -- Source filter -- */
function toggleSrc(name) {
  if (App.srcFilter.has(name)) App.srcFilter.delete(name);
  else App.srcFilter.add(name);
  lsSet('av-src-filter', JSON.stringify([...App.srcFilter]));
  renderSourceChips();
  renderAlerts();
  pushUrl();
}

function renderSourceChips() {
  const sources = App.data?.sources ?? [];
  const chips = sources.map(s => {
    const active = App.srcFilter.has(s.name);
    const count  = s.status === 'ok' ? `&thinsp;(${s.alert_count})` : '';
    return `<span class="src-flt-chip${active ? ' active' : ''}" data-src="${esc(s.name)}">` +
      `<span class="src-dot ${esc(s.status)}"></span>${esc(s.name)}${count}</span>`;
  }).join('');
  ['src-filter-chips', 'tv-src-chips'].forEach(id => {
    const el = document.getElementById(id);
    if (el) el.innerHTML = chips;
  });
}

/* -- Refresh -- */
document.getElementById('refresh-btn').addEventListener('click', () => fetchAlerts());

/* Refresh soon, coalescing bursts. Never fires while a fetch is in flight —
   fetchAlerts() would drop the call and the update would be lost. */
let sseRefreshTimer = null;
function scheduleRefresh(delay = 1000) {
  clearTimeout(sseRefreshTimer);
  sseRefreshTimer = setTimeout(() => {
    if (App.loading) scheduleRefresh(300);
    else fetchAlerts();
  }, delay);
}

/* -- Fetch -- */
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
      if (newAlerts.length) { 
        sendNotif(newAlerts); 
        playSoundForAlerts(newAlerts);
        newAlerts.forEach(a => App.freshFps.add(a.fingerprint)); 
      }
    }
    App.knownFps = curFps;
    saveKnownFps(curFps);
    
    // Update config from API response
    if (data.timezone) AppConfig.timezone = data.timezone;
    // `theme` holding a URL is the legacy way of declaring a custom stylesheet.
    applyCustomTheme(data.custom_css || data.theme);
    if (data.theme && THEME_PREFS.includes(data.theme) && !App.themeFromUser && !App.themeFromUrl) {
      applyTheme(data.theme, { persist: false });
    }
    if (data.play_sounds !== undefined) AppConfig.playSounds = data.play_sounds;
    
    App.data = data;
    applyTvDefault(data);

    render();

    const now = new Date().toLocaleTimeString('en-US');
    document.getElementById('last-refresh').textContent = now;
    document.getElementById('tv-last').textContent = now;

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


/* display.tv_mode_default only applies when this browser has no stored TV
   preference and the URL did not force one. */
function applyTvDefault(data) {
  if (!data.tv_mode_default || TV.chosen || App.tvFromUrl || TV.active) return;
  TV.active = true;
  TV._apply();
}

/* Split a query into label filters and free text. Comma-separated parts that
   look like `key=value` (also `!=` and `~` for "contains") become filters;
   anything else stays free text, so the plain search keeps working.
   Values can be OR-ed with "|", and repeating a key does the same.
   Example: "team=sre|dba, hostname~web, disk full" */
function parseQuery(q) {
  const filters = [];
  const text = [];
  for (const raw of q.split(',')) {
    const part = raw.trim();
    if (!part) continue;
    const m = part.match(/^([A-Za-z_][\w.\-\/]*)\s*(!=|=~|~|=)\s*(.+)$/);
    if (m) {
      const values = m[3].split('|').map(v => v.trim().toLowerCase()).filter(Boolean);
      if (values.length) filters.push({ key: m[1].toLowerCase(), op: m[2], values });
      else text.push(part);
    } else {
      text.push(part);
    }
  }
  return { filters, text: text.join(' ').toLowerCase() };
}

/* Value a filter key refers to: an alert field first, then its labels
   (case-insensitive, like the server does). */
function alertField(a, key) {
  switch (key) {
    case 'source': return a.source;
    case 'status': return a.status;
    case 'name':
    case 'alertname': return a.name;
  }
  const labels = a.labels ?? {};
  const hit = Object.keys(labels).find(l => l.toLowerCase() === key);
  if (hit) return labels[hit];
  if (key === 'severity') return a.severity;
  const ann = a.annotations ?? {};
  const annHit = Object.keys(ann).find(l => l.toLowerCase() === key);
  return annHit ? ann[annHit] : undefined;
}

function matchFilter(a, f) {
  const raw = alertField(a, f.key);
  // A label the alert does not carry only matches a negation.
  if (raw === undefined) return f.op === '!=';
  const value = String(raw).toLowerCase();
  const contains = f.op === '~' || f.op === '=~';
  const hit = f.values.some(v => (contains ? value.includes(v) : value === v));
  return f.op === '!=' ? !hit : hit;
}

/* Filters on the *same* key are OR-ed, filters on different keys AND-ed, so
   "team=sre, team=dba" reads as "either team" while "team=sre, severity=critical"
   still narrows. Negations are always AND-ed: "team!=sre, team!=dba" excludes
   both, which is the only reading that makes sense. */
function filtersMatch(a, filters) {
  const positives = new Map();
  for (const f of filters) {
    if (f.op === '!=') {
      if (!matchFilter(a, f)) return false;
    } else {
      if (!positives.has(f.key)) positives.set(f.key, []);
      positives.get(f.key).push(f);
    }
  }
  for (const group of positives.values()) {
    if (!group.some(f => matchFilter(a, f))) return false;
  }
  return true;
}

/* -- Filters -- */
function filteredAlerts() {
  const { filters, text } = parseQuery(App.searchQ || '');
  return (App.data?.alerts ?? []).filter(a => {
    if (!App.showSilenced && a.status === 'silenced') return false;
    if (App.sevFilter !== 'all' && a.severity !== App.sevFilter) return false;
    if (App.srcFilter.size > 0 && !App.srcFilter.has(a.source)) return false;
    if (filters.length && !filtersMatch(a, filters)) return false;
    if (text) {
      const hay = [a.name, a.severity, a.source, a.status,
        ...Object.values(a.labels ?? {}), ...Object.values(a.annotations ?? {})].join('\n').toLowerCase();
      if (!hay.includes(text)) return false;
    }
    return true;
  });
}

function toggleSev(s) {
  App.sevFilter = App.sevFilter === s ? 'all' : s;
  lsSet('av-sev-filter', App.sevFilter);
  renderStats();
  renderAlerts();
  TV.renderChips();
  pushUrl();
}

/* -- Render -- */
function updateTitle() {
  const firing = (App.data?.alerts ?? []).filter(a => a.status === 'firing');
  if (!firing.length) { document.title = 'AlertView'; return; }
  const bySev = s => firing.filter(a => a.severity === s).length;
  const icon = bySev('critical') ? '🔴' : bySev('error') ? '🟠' : bySev('high') ? '🟠' : '🟡';
  const alertWord = firing.length > 1 ? 'alerts' : 'alert';
  document.title = `${icon} ${firing.length} ${alertWord} — AlertView`;
}

function render() { renderStats(); renderSources(); renderSourceChips(); renderAlerts(); TV.renderChips(); TV.renderDots(); updateSilenceBtn(); updateTitle(); }

function renderStats() {
  const counts = {};
  (App.data?.alerts ?? []).forEach(a => { const s = a.severity || 'none'; counts[s] = (counts[s] || 0) + 1; });
  const order = Object.keys(counts).sort((a, b) => severityOrder(a) - severityOrder(b));
  document.getElementById('stats-bar').innerHTML = order
    .map(s => `<span class="stat-chip ${sevClass(s)}${App.sevFilter === s ? ' active' : ''}" data-sev="${esc(s)}">${counts[s]}&thinsp;${esc(s)}</span>`)
    .join('');
}

function renderSources() {
  document.getElementById('sources-bar').innerHTML = (App.data?.sources ?? []).map((s, i) => `
    ${i > 0 ? '<span class="src-sep">·</span>' : ''}
    <span class="src-item">
      <span class="src-dot ${s.status}"></span>
      ${esc(s.name)}
      ${s.status === 'ok' ? '· ' + s.alert_count + ' alert' + (s.alert_count !== 1 ? 's' : '') : `<span class="src-err-label" title="${esc(s.error)}">⚠ error</span>`}
    </span>`).join('');
}

/* Placeholder the server uses for an alert that lacks a grouping label. */
const MISSING_LABEL = '<missing>';

/* Build a single DOM element from an HTML string. */
function htmlToEl(html) {
  const t = document.createElement('template');
  t.innerHTML = html.trim();
  return t.content.firstElementChild;
}

/* Keyed DOM reconciliation: add/remove/replace/reorder only the children that
   actually changed, instead of rebuilding the whole container. Nodes are matched
   by a key stored on the element; a node is replaced only when its HTML differs. */
function reconcileChildren(container, items, getKey, getHtml) {
  const existing = new Map();
  for (const child of Array.from(container.children)) {
    if (child.__key != null) existing.set(child.__key, child);
  }

  const seen = new Set();
  let prev = null;
  for (const item of items) {
    const key = getKey(item);
    const html = getHtml(item);
    seen.add(key);

    let node = existing.get(key);
    if (!node) {
      node = htmlToEl(html);
    } else if (node.__html !== html) {
      const fresh = htmlToEl(html);
      node.replaceWith(fresh);
      node = fresh;
    }
    node.__key = key;
    node.__html = html;

    // Move into position only if it isn't already there.
    const ref = prev ? prev.nextSibling : container.firstChild;
    if (node !== ref) container.insertBefore(node, ref);
    prev = node;
  }

  // Drop anything no longer present (including non-keyed leftovers, e.g. the
  // empty-state placeholder when switching back to a populated list).
  for (const child of Array.from(container.children)) {
    if (child.__key == null || !seen.has(child.__key)) child.remove();
  }
}

function renderAlerts() {
  const filtered = filteredAlerts();
  const total    = App.data?.alerts.length ?? 0;
  const listEl   = document.getElementById('alert-list');

  const alertWord = total !== 1 ? 'alerts' : 'alert';
  document.getElementById('alert-count').textContent = filtered.length < total
    ? filtered.length + ' / ' + total + ' ' + alertWord
    : total + ' ' + alertWord;

  if (!filtered.length) {
    listEl.innerHTML = `<div class="empty-state">
      <div class="empty-state-icon">${App.searchQ || App.sevFilter !== 'all' || App.srcFilter.size > 0 ? '🔍' : '✅'}</div>
      <div>${App.searchQ ? 'No results for &laquo;&nbsp;' + esc(App.searchQ) + '&nbsp;&raquo;' : 'No active alerts'}</div>
    </div>`;
    return;
  }

  // Check if grouping is enabled and we have groups
  const groups  = App.data?.groups || [];
  const groupBy = App.data?.group_by || [];

  if (groups.length > 0 && groupBy.length > 0) {
    renderGroupedAlerts(listEl, groups, filtered);
  } else {
    reconcileChildren(listEl, filtered, a => a.fingerprint, cardHtml);
  }

  // Highlight freshly arrived alerts (and clear the highlight from the rest).
  listEl.querySelectorAll('.alert-card').forEach(el => {
    el.classList.toggle('new', App.freshFps.has(el.dataset.fp));
  });
}

/* Membership comes from the labels the server sends, not from re-parsing the
   group key: a value containing "," or "=" used to scramble the split, and the
   "<missing>" placeholder was compared as if it were a real label value, so
   alerts without the grouping label matched nothing and vanished. */
function alertsInGroup(group, filtered) {
  const entries = Object.entries(group.labels || {});
  return filtered.filter(a => entries.every(([key, value]) =>
    value === MISSING_LABEL ? a.labels?.[key] === undefined : a.labels?.[key] === value));
}

function groupSevBadges(group) {
  return Object.entries(group.severity_counts || {})
    .sort(([a], [b]) => severityOrder(a) - severityOrder(b))
    .map(([sev, count]) => `<span class="sev-badge ${sevClass(sev)}">${count} ${esc(sev)}</span>`)
    .join('');
}

/* Group shell (header + empty body); cards are reconciled separately so that
   expand/collapse state and individual cards survive a refresh. */
function groupShellHtml(group) {
  const groupLabel = Object.entries(group.labels || {})
    .map(([key, value]) => `<span class="lbl">${esc(key)}=<b>${esc(value)}</b></span>`)
    .join('');

  return `
    <div class="alert-group" data-group-key="${esc(group.key)}">
      <div class="group-header">
        <span class="group-toggle">▶</span>
        <span class="group-label">${groupLabel}</span>
        <span class="group-count"></span>
        <span class="group-severities"></span>
      </div>
      <div class="group-alerts" id="group-${esc(group.key)}" style="display:none;"></div>
    </div>`;
}

function applyGroupOpen(groupEl, isOpen) {
  groupEl.querySelector('.group-alerts').style.display = isOpen ? 'block' : 'none';
  groupEl.querySelector('.group-toggle').textContent = isOpen ? '▼' : '▶';
}

function renderGroupedAlerts(listEl, groups, filtered) {
  const visible = groups
    .map(group => ({ group, alerts: alertsInGroup(group, filtered) }))
    .filter(x => x.alerts.length > 0);

  const existing = new Map();
  for (const child of Array.from(listEl.children)) {
    if (child.__groupKey != null) existing.set(child.__groupKey, child);
  }

  const seen = new Set();
  let prev = null;
  for (const { group, alerts } of visible) {
    seen.add(group.key);

    let groupEl = existing.get(group.key);
    if (!groupEl) {
      groupEl = htmlToEl(groupShellHtml(group));
      groupEl.__groupKey = group.key;
    }

    // Update header counts in place (no full rebuild).
    const countEl = groupEl.querySelector('.group-count');
    const countTxt = `${alerts.length} alert${alerts.length !== 1 ? 's' : ''}`;
    if (countEl.textContent !== countTxt) countEl.textContent = countTxt;
    const sevEl = groupEl.querySelector('.group-severities');
    const sevHtml = groupSevBadges(group);
    if (sevEl.innerHTML !== sevHtml) sevEl.innerHTML = sevHtml;

    // Reconcile the cards inside the group, then restore open/closed state.
    reconcileChildren(groupEl.querySelector('.group-alerts'), alerts, a => a.fingerprint, cardHtml);
    applyGroupOpen(groupEl, App.openGroups.has(group.key));

    const ref = prev ? prev.nextSibling : listEl.firstChild;
    if (groupEl !== ref) listEl.insertBefore(groupEl, ref);
    prev = groupEl;
  }

  for (const child of Array.from(listEl.children)) {
    if (child.__groupKey == null || !seen.has(child.__groupKey)) child.remove();
  }
}

let sevOrderCache = { source: null, canon: [] };

function severityOrder(sev) {
  const source = sevOrderList();
  // Same array identity until a new payload arrives, so this maps once per
  // refresh instead of once per comparison in every sort.
  if (sevOrderCache.source !== source) {
    sevOrderCache = { source, canon: source.map(canonSev) };
  }
  const i = sevOrderCache.canon.indexOf(canonSev(sev));
  return i === -1 ? sevOrderCache.canon.length : i;
}

function toggleGroup(groupKey, groupEl) {
  if (App.openGroups.has(groupKey)) App.openGroups.delete(groupKey);
  else App.openGroups.add(groupKey);

  const el = groupEl || document.getElementById('group-' + groupKey)?.closest('.alert-group');
  if (el) applyGroupOpen(el, App.openGroups.has(groupKey));
}

function getSourceLabel(sourceType) {
  const labels = {
    alertmanager: "Open in Alertmanager",
    grafana: "Open in Grafana",
    zabbix: "Open in Zabbix"
  };
  return labels[sourceType] || "Open in source";
}

function linkTarget() {
  return App.data?.link_new_tab === false ? '' : ' target="_blank" rel="noopener noreferrer"';
}

/* The alert link hangs off the severity marker rather than the whole card: one
   small, deliberate target that says what it does on hover, instead of a row
   that navigates wherever you happen to click. */
function severityMarkLink(a, mark) {
  if (!a.alert_link_url) return mark;
  return `<a class="mark-link" href="${esc(a.alert_link_url)}"${linkTarget()}` +
    ` title="${esc(a.name)} — open the runbook">${mark}</a>`;
}

function genLinkHtml(url, sourceType, sourceName) {
  if (!url) return '';
  const label = sourceType ? getSourceLabel(sourceType) : "Open in Prometheus/Grafana";
  const title = sourceName ? `${sourceName} — ${label}` : label;
  return `<a href="${esc(url)}"${linkTarget()} class="gen-link" title="${esc(title)}">
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
      <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
      <polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/>
    </svg></a>`;
}

/* Labels shown in front of the alert name, joined by the configured separator.
   Only the ones the alert carries; empty string when it carries none. */
function prefixHtml(a) {
  const sep = App.data?.prefix_separator ?? ' / ';
  const parts = prefixLabels()
    .filter(l => a.labels?.[l] !== undefined)
    .map(l => esc(a.labels[l]));
  if (!parts.length) return '';
  return `<span class="alert-prefix">` +
    parts.join(`<span class="prefix-sep">${esc(sep)}</span>`) +
    `</span>`;
}

function prefixLabels() {
  return App.data?.prefix_labels ?? [];
}

/* Labels for the chips: the configured ones the alert carries, minus the ones
   already shown in the prefix. */
function chipLabels(a) {
  const prefix = prefixLabels();
  return (App.data?.display_labels ?? [])
    .filter(l => a.labels?.[l] !== undefined && l !== 'alertname' && l !== 'severity')
    .filter(l => !prefix.includes(l));
}

/* The main text of an alert: its name, or the summary when the config hides
   the name. An alert with no summary keeps its name rather than showing
   nothing. Returns the text and whether the summary was consumed by it. */
/* What an alert shows inline, what sits behind the toggle, and whether the
   toggle is open. Everything the config hides — the label chips with
   show_labels: false, the alert name with show_alert_name: false — goes behind
   the toggle rather than disappearing, so it is always one click away. The open
   state is keyed by fingerprint so it survives a refresh: the card is
   re-rendered every 30s. */
function labelLayout(a, inlineCount) {
  const labels = chipLabels(a);
  const inline = App.data?.show_labels === false ? 0 : inlineCount;
  const hidden = labels.slice(inline).map(l => ({ key: l, value: a.labels[l] }));

  // The alert name is not a chip; when the summary takes its place it would be
  // nowhere to be seen, so it leads the hidden list.
  if (App.data?.show_alert_name === false && a.name) {
    hidden.unshift({ key: 'alertname', value: a.name });
  }

  return {
    visible: labels.slice(0, inline).map(l => ({ key: l, value: a.labels[l] })),
    hidden,
    open: App.openLabels.has(a.fingerprint),
  };
}

function labelChip({ key, value }, extraClass = '') {
  return `<span class="lbl${extraClass}">${esc(key)}=<b>${esc(value)}</b></span>`;
}

function hiddenLabelsHtml(hidden, open, extraClass = '') {
  if (!hidden.length) return '';
  const chips = hidden.map(i => labelChip(i, extraClass)).join('');
  return `<span class="hidden-labels"${open ? '' : ' style="display:none"'}>${chips}</span>`;
}

function labelsToggleHtml(hidden, open) {
  if (!hidden.length) return '';
  return `<button class="labels-toggle" data-labels-toggle title="${open ? 'Hide' : 'Show'} details">` +
    `${open ? '−' : '+' + hidden.length}</button>`;
}

function alertTitle(a) {
  const summary = a.annotations?.summary || '';
  if (App.data?.show_alert_name === false && summary) {
    return { text: summary, usedSummary: true };
  }
  return { text: a.name, usedSummary: false };
}

function criticalIcon(a) {
  const icon = App.data?.critical_icon;
  if (!icon || canonSev(a.severity) !== 'critical') return '';
  return `<span class="crit-icon" aria-hidden="true">${esc(icon)}</span>`;
}

/* The severity marker leading an alert: the critical icon replaces the dot
   rather than sitting next to it, so a critical stands out at a glance. Other
   severities keep their coloured dot, and so does a critical when
   display.critical_icon is empty — the alert would otherwise lead with nothing. */
function severityMark(a) {
  const mark = criticalIcon(a) || `<span class="sev-dot ${sevClass(a.severity || 'none')}"></span>`;
  return severityMarkLink(a, mark);
}

/* Status is a badge no more: `firing` is the norm and saying so on every row is
   noise. Only the exceptions get a marker, from display.status_icons. */
function statusMark(a) {
  const icon = App.data?.status_icons?.[a.status];
  if (!icon) return '';
  return `<span class="status-icon" title="${esc(a.status)}">${esc(icon)}</span>`;
}

/* The silence or acknowledgement comment, behind its own button. */
function alertComment(a) {
  return a.annotations?.acknowledgement || a.annotations?.silence_comment || '';
}

/* Who silenced or acknowledged the alert: the silence's createdBy for
   Alertmanager and Grafana, the acknowledging user for Zabbix. */
function commentAuthor(a) {
  return a.annotations?.silence_created_by || a.labels?.acknowledged_by || '';
}

function commentToggleHtml(a) {
  if (!alertComment(a)) return '';
  const open = App.openComments.has(a.fingerprint);
  const author = commentAuthor(a);
  const what = author ? `the comment from ${author}` : 'the comment';
  return `<button class="comment-toggle${open ? ' active' : ''}" data-comment-toggle` +
    ` title="${esc(open ? 'Hide ' + what : 'Show ' + what)}">💬</button>`;
}

function commentHtml(a) {
  const comment = alertComment(a);
  if (!comment) return '';
  const open = App.openComments.has(a.fingerprint);
  const author = commentAuthor(a);
  return `<span class="row-comment"${open ? '' : ' style="display:none"'}>` +
    (author ? `<b class="comment-author">${esc(author)}</b> ` : '') +
    esc(comment) + `</span>`;
}

function cardHtml(a) {
  if (TV.active) return cardHtmlTV(a);

  const sev    = a.severity || 'none';
  // A card has room for every label, so nothing is behind the toggle unless the
  // config hides them.
  const lay    = labelLayout(a, Infinity);
  const labels = lay.visible.map(i => labelChip(i)).join('')
    + labelsToggleHtml(lay.hidden, lay.open)
    + hiddenLabelsHtml(lay.hidden, lay.open);

  const title    = alertTitle(a);
  const summary  = title.usedSummary ? '' : (a.annotations?.summary || '');
  const desc     = a.annotations?.description || '';
  const showDesc = desc && desc !== title.text && desc !== summary;

  return `
    <div class="alert-card ${sevClass(sev)}" data-fp="${esc(a.fingerprint)}">
      <div class="card-top">
        <div class="card-title">
          ${severityMark(a)}
          ${prefixHtml(a)}
          <span class="sev-badge ${sevClass(sev)}">${esc(sev)}</span>
          ${statusMark(a)}${commentToggleHtml(a)}
          <span class="alert-name${title.usedSummary ? ' is-summary' : ''}">${esc(title.text)}</span>
        </div>
        <div class="card-meta">
          <span class="src-chip">${esc(a.source)}</span>
          <span class="time-ago" title="${esc(absTime(a.starts_at))}">for&nbsp;${relTime(a.starts_at)}</span>
          ${genLinkHtml(a.link_url, a.source_type)}
        </div>
      </div>
      ${summary  ? `<div class="card-summary">${esc(summary)}</div>` : ''}
      ${showDesc ? `<div class="card-desc">${esc(desc)}</div>` : ''}
      ${commentHtml(a)}
      ${labels   ? `<div class="label-chips">${labels}</div>` : ''}
    </div>`;
}

function cardHtmlTV(a) {
  const sev     = a.severity || 'none';
  const title   = alertTitle(a);
  const summary = title.usedSummary ? '' : (a.annotations?.summary || '');
  
  // A row only has space for 2 labels inline, the rest go behind the +N toggle.
  // Presence is filtered *before* slicing so a row never hides every label it
  // has; labels already shown in the prefix are excluded by chipLabels().
  const lay = labelLayout(a, 2);
  const labelsHtml = lay.visible.map(i => labelChip(i, ' tv-lbl')).join('');

  // Every slot is always emitted, even empty: the row is a subgrid of the list,
  // so a missing element would shift the columns of that row only. This is what
  // lines the rows up — see the "TV mode: rows share one grid" block in the CSS.
  return `
    <div class="alert-card alert-row ${sevClass(sev)}" data-fp="${esc(a.fingerprint)}">
      <span class="row-lead">${severityMark(a)}</span>
      ${prefixHtml(a) || '<span class="alert-prefix"></span>'}
      <span class="sev-badge ${sevClass(sev)}">${esc(sev)}</span>
      <span class="row-status">${statusMark(a)}${commentToggleHtml(a)}</span>
      <span class="alert-name${title.usedSummary ? ' is-summary' : ''}">${esc(title.text)}</span>
      <span class="row-summary">${esc(summary)}</span>
      <span class="row-labels">${labelsHtml}${labelsToggleHtml(lay.hidden, lay.open)}</span>
      <span class="time-ago" title="${esc(absTime(a.starts_at))}">for&nbsp;${relTime(a.starts_at)}</span>
      <span class="row-link">${genLinkHtml(a.link_url, a.source_type, a.source)}</span>
      ${hiddenLabelsHtml(lay.hidden, lay.open, ' tv-lbl')}
      ${commentHtml(a)}
    </div>`;
}

/* -- TV Mode -- */
const TV = {
  active:     false,
  panelOpen:  false,
  moreOpen:   false,
  clockTimer: null,

  init() {
    // No stored preference means the config default applies, but the payload
    // has not arrived yet — see applyTvDefault().
    this.chosen = lsGet('av-tv') !== null;
    this.active = lsGet('av-tv') === 'true';
    if (this.active) this._apply();

    document.getElementById('tv-btn').addEventListener('click',      () => this.toggle());
    document.getElementById('tv-settings-btn').addEventListener('click', e => { e.stopPropagation(); this.togglePanel(); });
    document.getElementById('tv-exit-btn').addEventListener('click', () => this.toggle());
    document.getElementById('tv-more-btn').addEventListener('click', e => { e.stopPropagation(); this.toggleMore(); });

    // Close panel when clicking elsewhere
    document.addEventListener('click', e => {
      if (this.panelOpen && !document.getElementById('tv-panel').contains(e.target) && e.target.id !== 'tv-settings-btn') {
        this.closePanel();
      }
    });

    // Keyboard shortcuts
    document.addEventListener('keydown', e => {
      if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT') return;
      if (e.key === 'Escape') {
        if (this.panelOpen) this.closePanel();
        else if (this.moreOpen) this.closeMore();
        else if (this.active) this.toggle();
      }
      if ((e.key === 't' || e.key === 'T') && !e.ctrlKey && !e.metaKey) this.toggle();
    });
  },

  toggle() {
    this.active = !this.active;
    this.chosen = true;
    lsSet('av-tv', this.active);
    this._apply();
    pushUrl();
  },

  _apply() {
    document.documentElement.setAttribute('data-tv', this.active);
    document.getElementById('tv-btn').classList.toggle('active', this.active);
    if (this.active) {
      this.startClock();
      this.renderChips();
      this.renderDots();
      renderSourceChips();
    } else {
      this.stopClock();
      this.closePanel();
      this.closeMore();
    }
    renderAlerts();
  },

  startClock() {
    this.stopClock(); // never stack two intervals
    this.updateClock();
    this.clockTimer = setInterval(() => this.updateClock(), 1000);
  },
  stopClock() { clearInterval(this.clockTimer); },
  updateClock() {
    const options = { hour: '2-digit', minute: '2-digit', second: '2-digit' };
    if (AppConfig.timezone !== 'local') {
      options.timeZone = AppConfig.timezone;
    }
    document.getElementById('tv-clock').textContent =
      new Date().toLocaleTimeString('en-US', options);
  },

  /* The controls half of the HUD. The minimal half — dots, clock, last refresh,
     version — is always on, so there is no bar-wide hide timer any more. */
  toggleMore() {
    this.moreOpen ? this.closeMore() : this.openMore();
  },
  openMore() {
    this.moreOpen = true;
    document.getElementById('tv-bar-more').classList.add('open');
    document.getElementById('tv-more-btn').textContent = '−';
  },
  closeMore() {
    this.moreOpen = false;
    document.getElementById('tv-bar-more').classList.remove('open');
    document.getElementById('tv-more-btn').textContent = '+';
    this.closePanel();
  },

  togglePanel() {
    this.panelOpen ? this.closePanel() : this.openPanel();
  },
  openPanel() {
    this.panelOpen = true;
    document.getElementById('tv-panel').classList.add('open');
  },
  closePanel() {
    this.panelOpen = false;
    document.getElementById('tv-panel').classList.remove('open');
  },

  renderChips() {
    const counts = {};
    (App.data?.alerts ?? []).forEach(a => { const s = a.severity || 'none'; counts[s] = (counts[s] || 0) + 1; });
    const order = Object.keys(counts).sort((a, b) => severityOrder(a) - severityOrder(b));
    const all = `<span class="stat-chip${App.sevFilter === 'all' ? ' active' : ''}" style="font-size:10px;padding:1px 7px" data-sev="all">all</span>`;
    document.getElementById('tv-sev-chips').innerHTML = all + order
      .map(s => `<span class="stat-chip ${sevClass(s)}${App.sevFilter === s ? ' active' : ''}" style="font-size:10px;padding:1px 7px" data-sev="${esc(s)}">${counts[s]}&thinsp;${esc(s)}</span>`)
      .join('');
  },

  renderDots() {
    document.getElementById('tv-dots').innerHTML = (App.data?.sources ?? [])
      .map(s => `<span class="src-dot ${s.status}" title="${esc(s.name)}${s.error ? ': ' + esc(s.error) : ''}"></span>`)
      .join('');
  },
};

TV.init();

/* -- URL state sync -- */
function pushUrl() {
  const p = new URLSearchParams();
  if (App.themePref !== 'auto') p.set('theme', App.themePref);
  if (App.sevFilter !== 'all') p.set('sev',      App.sevFilter);
  if (App.srcFilter.size > 0)  p.set('src', [...App.srcFilter].join(','));
  if (App.searchQ)             p.set('q',        App.searchQ);
  if (App.showSilenced)        p.set('silenced', '1');
  if (TV.active)               p.set('tv',       '1');
  const qs = p.toString();
  history.replaceState(null, '', qs ? '?' + qs : location.pathname);
}

/* URL parameters apply to this visit only. They used to be written to
   localStorage, so opening a shared "?tv=1" or "?sev=critical" link once pinned
   that setting in the visitor's browser for good. */
function initFromUrl() {
  const p = new URLSearchParams(location.search);
  if (p.has('theme')) { App.themeFromUrl = true; applyTheme(p.get('theme'), { persist: false }); }
  if (p.has('sev'))   App.sevFilter = p.get('sev');
  if (p.has('src'))   App.srcFilter = new Set(p.get('src').split(',').filter(Boolean));
  if (p.has('q'))     {
    App.searchQ = p.get('q');
    SearchInput.value = App.searchQ;
    SearchClear.style.display = App.searchQ ? 'block' : 'none';
  }
  if (p.has('silenced')) App.showSilenced = p.get('silenced') === '1';
  if (p.has('tv')) {
    App.tvFromUrl = true;
    const on = p.get('tv') === '1';
    if (on !== TV.active) { TV.active = on; TV._apply(); }
  }
}

/* Chips and group headers are rebuilt on every render, so the handlers live on
   the containers. Inline onclick attributes carried alert-controlled values
   (severity, source name, group key) straight into an HTML attribute and a JS
   string literal — a quote in any of them broke out of both. */
function delegate(containerId, selector, handler) {
  const el = document.getElementById(containerId);
  if (el) el.addEventListener('click', e => {
    const hit = e.target.closest(selector);
    if (hit && el.contains(hit)) handler(hit, e);
  });
}

['stats-bar', 'tv-sev-chips'].forEach(id =>
  delegate(id, '[data-sev]', el => toggleSev(el.dataset.sev)));
['src-filter-chips', 'tv-src-chips'].forEach(id =>
  delegate(id, '[data-src]', el => toggleSrc(el.dataset.src)));
delegate('alert-list', '[data-comment-toggle]', (el, e) => {
  e.preventDefault();
  e.stopPropagation();
  const fp = el.closest('.alert-card')?.dataset.fp;
  if (!fp) return;
  if (App.openComments.has(fp)) App.openComments.delete(fp);
  else App.openComments.add(fp);
  renderAlerts();
});
delegate('alert-list', '[data-labels-toggle]', (el, e) => {
  // The card-wide link overlay sits under this button; stop the click there.
  e.preventDefault();
  e.stopPropagation();
  const fp = el.closest('.alert-card')?.dataset.fp;
  if (!fp) return;
  if (App.openLabels.has(fp)) App.openLabels.delete(fp);
  else App.openLabels.add(fp);
  renderAlerts();
});
delegate('alert-list', '.group-header', el => {
  const groupEl = el.closest('.alert-group');
  if (groupEl) toggleGroup(groupEl.dataset.groupKey, groupEl);
});

/* Ctrl+F / Cmd+F and "/" focus the search box instead of the browser's
   find-in-page, which only ever finds what is already on screen. Not in TV
   mode: the header is hidden there, so the native search stays available. */
document.addEventListener('keydown', e => {
  const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(e.target.tagName) || e.target.isContentEditable;
  const wants = (e.key === 'f' || e.key === 'F') ? (e.ctrlKey || e.metaKey)
              : (e.key === '/' && !typing && !e.ctrlKey && !e.metaKey);
  if (!wants || TV.active) return;
  e.preventDefault();
  SearchInput.focus();
  SearchInput.select();
});

// Escape leaves the search box, clearing it when it is empty of intent.
SearchInput.addEventListener('keydown', e => {
  if (e.key !== 'Escape') return;
  if (App.searchQ) {
    SearchInput.value = App.searchQ = '';
    SearchClear.style.display = 'none';
    renderAlerts();
    pushUrl();
  } else {
    SearchInput.blur();
  }
});

/* -- Boot -- */
// The <head> script already resolved the theme to avoid a flash; this syncs the
// button icons and the theme-color meta with it.
applyTheme(App.themePref, { persist: false });
initFromUrl();
updateSilenceBtn();
fetchAlerts();
