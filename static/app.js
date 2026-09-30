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

/* Escapes for both text and attribute contexts. The previous implementation
   round-tripped through textContent/innerHTML, which is the DOM's *text node*
   serialisation: it escapes & < >, and leaves quotes alone. Everything here is
   interpolated into HTML strings, attributes included (title=, data-sev=,
   data-group-key=), so an alert label holding a double quote closed the
   attribute and the next word became a live event handler. */
const ESC = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };

function esc(s) {
  if (s == null) return '';
  return String(s).replace(/[&<>"']/g, c => ESC[c]);
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

/* The configured timezone, validated once. Intl throws on an IANA name with a
   typo, and the TV clock had no guard: it died on its first tick and threw
   once a second afterwards. Falling back to the browser's zone beats a frozen
   clock on a wall display. */
let tzChecked = null;

function tzOptions() {
  const tz = AppConfig.timezone;
  if (!tz || tz === 'local') return {};
  if (tzChecked && tzChecked.tz === tz) return tzChecked.options;
  let options = { timeZone: tz };
  try {
    new Date().toLocaleString('en-US', options);
  } catch {
    console.warn(`AlertView: unknown timezone "${tz}", using the browser's instead`);
    options = {};
  }
  tzChecked = { tz, options };
  return options;
}

function absTime(iso) {
  try {
    return new Date(iso).toLocaleString('en-US', tzOptions());
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

/* The page's Content-Security-Policy allows the custom stylesheet's host as it
   was when the page was served, and a policy cannot change without a reload. A
   hot reload of the config pointing at another host would be refused by the
   browser, silently, until someone reloaded the page by hand — on a wall
   display, never. Only a change of host needs it: the same host, or removing
   the stylesheet, applies in place. */
function cssOrigin(url) {
  if (!url || THEME_PREFS.includes(url)) return null;
  try {
    const origin = new URL(url, location.href).origin;
    return origin === location.origin ? null : origin;
  } catch { return null; }
}

function stylesheetNeedsReload(cssUrl) {
  const origin = cssOrigin(cssUrl);
  if (App.cssOrigin === undefined) { App.cssOrigin = origin; return false; }
  return origin !== null && origin !== App.cssOrigin;
}

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

document.getElementById('config-dismiss')?.addEventListener('click', () => showConfigError(null));
document.getElementById('theme-btn').addEventListener('click', cycleTheme);
document.getElementById('tv-theme-btn').addEventListener('click', cycleTheme);

/* -- knownFps persistence --
   Kept per source, the way the server does it: a source that is pending or
   failing keeps what it had, and one seen for the first time is primed
   silently. With a single set, a response missing a source — at startup, or
   during an outage — made its whole backlog "new" once it came back. */
function loadKnownFps() {
  try {
    const raw = lsGet('av-known-fps');
    if (!raw) return null;
    const { bySource, ts } = JSON.parse(raw);
    if (!bySource || Date.now() - ts > 86400000) return null;
    return new Map(Object.entries(bySource).map(([name, fps]) => [name, new Set(fps)]));
  } catch { return null; }
}

function saveKnownFps(known) {
  try {
    const bySource = Object.fromEntries([...known].map(([name, fps]) => [name, [...fps]]));
    lsSet('av-known-fps', JSON.stringify({ bySource, ts: Date.now() }));
  } catch {}
}

/* The alerts not seen before, and what is known after this response. Only a
   source that answered ("ok") and was already known can announce anything. */
function diffKnown(known, data) {
  const bySource = new Map();
  for (const a of data.alerts) {
    if (!bySource.has(a.source)) bySource.set(a.source, []);
    bySource.get(a.source).push(a);
  }
  const next = new Map();
  const fresh = [];
  for (const s of data.sources ?? []) {
    if (s.status !== 'ok') {
      if (known?.has(s.name)) next.set(s.name, known.get(s.name));
      continue;
    }
    const alerts = bySource.get(s.name) ?? [];
    const seen = known?.get(s.name);
    if (seen) fresh.push(...alerts.filter(a => !seen.has(a.fingerprint)));
    next.set(s.name, new Set(alerts.map(a => a.fingerprint)));
  }
  return { next, fresh };
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
let sseRetryCount = 0;
const maxSseRetries = 5;

function connectSSE() {
  if (!('EventSource' in window)) {
    console.log('SSE not supported in this browser');
    return;
  }

  const eventSource = new EventSource('/events');
  
  eventSource.onopen = () => {
    sseRetryCount = 0;
    console.log('SSE connection opened');
  };

  eventSource.onerror = (err) => {
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
    showConfigError(null);
    scheduleRefresh();
  });

  /* The server refused an edit and kept the previous configuration. Nothing on
     screen used to say so, which on a wall display means the change looks
     applied and is not. */
  eventSource.addEventListener('config_error', e => {
    console.warn('AlertView: configuration rejected —', e.data);
    showConfigError(e.data);
  });
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

/* The tone for a severity. A level with no preset of its own borrows the
   nearest one in the configured order, more severe first: the loop used to
   stop at the most severe level present and play nothing at all when that
   level had no preset — which is every custom severity. */
function presetFor(sev) {
  const order = sevOrderList().map(canonSev);
  const start = order.indexOf(canonSev(sev));
  if (start === -1) return SOUND_PRESETS[canonSev(sev)] || null;
  for (let d = 0; d < order.length; d++) {
    if (start - d >= 0 && SOUND_PRESETS[order[start - d]]) return SOUND_PRESETS[order[start - d]];
    if (start + d < order.length && SOUND_PRESETS[order[start + d]]) return SOUND_PRESETS[order[start + d]];
  }
  // Nobody in this severity_order has a preset — a fully renamed scale. Map
  // the rank onto the built-in one so the top of the scale still sounds more
  // urgent than the bottom, instead of the page going silent.
  const defaults = DEFAULT_SEV_ORDER.filter(s => SOUND_PRESETS[s]);
  if (!defaults.length) return null;
  const span = Math.max(1, order.length - 1);
  return SOUND_PRESETS[defaults[Math.min(defaults.length - 1,
    Math.round((start / span) * (defaults.length - 1)))]];
}

function playSoundForAlerts(newAlerts) {
  if (!AppConfig.playSounds || !newAlerts.length) return;
  const worst = newAlerts.reduce((best, a) =>
    severityOrder(a.severity) < severityOrder(best.severity) ? a : best);
  const preset = presetFor(worst.severity);
  if (preset) preset();
}

/* Icon standing for a batch of alerts. Driven by the configured severity
   order: keyed off the level *names* it was always 🟡 as soon as someone
   renamed their severities. */
function severityIcon(list) {
  const worst = list.reduce((rank, a) => Math.min(rank, severityOrder(a.severity)), Infinity);
  return worst === 0 ? '🔴' : worst <= 2 ? '🟠' : '🟡';
}

function sendNotif(newAlerts) {
  if (Notification?.permission !== 'granted' || !newAlerts.length) return;
  const icon = severityIcon(newAlerts);
  const alertWord = newAlerts.length > 1 ? 'alerts' : 'alert';
  const n = new Notification(
    `${icon} ${newAlerts.length} new ${alertWord}`,
    { body: newAlerts.slice(0, 6).map(a => `[${a.severity.toUpperCase()}] ${a.name}`).join('\n') + (newAlerts.length > 6 ? `\n... and ${newAlerts.length - 6} more` : '') }
  );
  n.onclick = () => { window.focus(); n.close(); };
  setTimeout(() => n.close(), 9000);
}

/* A stored or URL-borne filter selection: a comma-separated list of names, where
   the empty set means "everything". `all` is spelled out rather than left empty
   so a URL can say it explicitly, and so a value written by an older version —
   which stored one name, or the literal "all" — still reads correctly. */
function parseFilterSet(raw) {
  if (!raw || raw === 'all') return new Set();
  return new Set(raw.split(',').map(s => s.trim()).filter(Boolean));
}

/* The reverse, for localStorage and the URL. */
function formatFilterSet(set) {
  return set.size ? [...set].join(',') : 'all';
}

/* -- App state (filters persisted in localStorage) -- */
const App = {
  data:           null,
  themePref:      lsGet('av-theme') || 'auto',
  themeFromUser:  lsGet('av-theme') !== null,
  themeFromUrl:   false,
  tvFromUrl:      false,
  knownFps:       loadKnownFps(),
  cssOrigin:      undefined,   // custom stylesheet host the page's CSP allows
  freshFps:       new Set(),
  searchQ:        '',
  /* Severities and statuses are sets, not single values: several chips can be on
     at once, as the source chips always allowed. An empty set means "no filter",
     which is what the `all` chip selects. A value stored by an older version is a
     single name, and splitting it on commas turns it into a one-element set. */
  sevFilter:      parseFilterSet(lsGet('av-sev-filter')),
  srcFilter:      (() => { try { const r = lsGet('av-src-filter'); return new Set(r ? JSON.parse(r) : []); } catch { return new Set(); } })(),
  statusFilter:   (() => {
    const raw = lsGet('av-status-filter');
    // Nothing stored: fall back to the boolean this replaced, where showing
    // silenced alerts meant showing everything.
    if (raw === null) return new Set(lsGet('av-show-silenced') === 'true' ? [] : ['firing']);
    return parseFilterSet(raw);
  })(),
  refreshTimer:   null,
  lastSuccess:    null,
  stale:          false,
  countdownTimer: null,
  countdown:      0,
  loading:        false,
  openGroups:     new Set(),
  openLabels:     new Set(),
  openComments:   new Set(),
};

/* -- Search -- */
/* Two fields, one query: the header box, and its twin in the TV panel. The
   header is display:none in TV mode, so the twin is the only reachable one
   there. Same arrangement as the silence and theme buttons. */
const SearchInput = document.getElementById('search');
const SearchClear = document.getElementById('search-clear');
const SearchInputs = [SearchInput, document.getElementById('tv-search')].filter(Boolean);

/* Mirrors App.searchQ into whichever field is not the one being typed in.
   Assigning to .value unconditionally would move the caret to the end on every
   keystroke, so only a field that actually differs is written. */
function syncSearchInputs() {
  SearchInputs.forEach(el => { if (el.value !== App.searchQ) el.value = App.searchQ; });
  SearchClear.style.display = App.searchQ ? 'block' : 'none';
}

/* Re-render after the typing settles. Every keystroke used to rebuild the whole
   list synchronously, which is fine for a dozen alerts and not for a wall
   display carrying hundreds. */
let searchTimer = null;
function onSearchChanged(immediate = false) {
  syncSearchInputs();
  clearTimeout(searchTimer);
  const apply = () => { renderAlerts(); pushUrl(); };
  if (immediate) apply();
  else searchTimer = setTimeout(apply, 120);
}

/* The one place the search is emptied — the body of this was written out three
   times, and the copies had already started to differ. */
function clearSearch() {
  App.searchQ = '';
  onSearchChanged(true);
}

SearchInputs.forEach(el => el.addEventListener('input', e => {
  App.searchQ = e.target.value;
  onSearchChanged();
}));
SearchClear.addEventListener('click', clearSearch);

/* -- Suppressed-alert filter --
   A silence is somebody's decision; an inhibition is a consequence of another
   alert firing. This used to be one show/hide toggle covering both, so "what am
   I masking right now?" could not be asked. `firing` stands for "not
   suppressed", so a pending alert rides along with it. */
const STATUS_KINDS = ['firing', 'silenced', 'inhibited'];
const STATUS_CHIPS = [
  ['firing',    'Firing',    'Alerts that are neither silenced nor inhibited'],
  ['silenced',  'Silenced',  'Alerts someone has silenced'],
  ['inhibited', 'Inhibited', 'Alerts another alert is masking'],
  ['all',       'All',       'Every alert, suppressed or not'],
];

/* A stored value from another version, or a hand-edited one, must not leave the
   page filtering on a status that cannot exist. Pruning everything away would
   read as "no filter", which is not what a stored selection meant, so that case
   goes back to the default rather than quietly showing the suppressed alerts. */
{
  const known = [...App.statusFilter].filter(m => STATUS_KINDS.includes(m));
  if (known.length !== App.statusFilter.size) {
    App.statusFilter = new Set(known.length ? known : ['firing']);
  }
}

/* One chip in or out of a selection. Every chip row works this way — severities,
   statuses and sources alike: a click adds that one or takes it out, so several
   can be on at once with no modifier to hold down. An empty set means "no
   filter", which is also what the `all` chip selects. */
function applyChipSelection(set, value) {
  if (value === 'all') set.clear();
  else if (set.has(value)) set.delete(value);
  else set.add(value);
}

function renderStatusChips() {
  const chips = STATUS_CHIPS.map(([mode, label, title]) => {
    const active = mode === 'all' ? App.statusFilter.size === 0 : App.statusFilter.has(mode);
    return `<span class="status-flt-chip${active ? ' active' : ''}" data-status="${mode}"` +
      ` role="button" tabindex="0" aria-pressed="${active}" title="${esc(title)}">${label}</span>`;
  }).join('');
  ['status-filter-chips', 'tv-status-chips'].forEach(id => {
    const el = document.getElementById(id);
    if (el) el.innerHTML = chips;
  });
}

function setStatusFilter(mode) {
  if (mode !== 'all' && !STATUS_KINDS.includes(mode)) return;
  applyChipSelection(App.statusFilter, mode);
  lsSet('av-status-filter', formatFilterSet(App.statusFilter));
  renderStatusChips();
  renderAlerts();
  pushUrl();
}

/* -- Source filter --
   The row that set the pattern the other two now follow. */
function toggleSrc(name) {
  applyChipSelection(App.srcFilter, name);
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
    return `<span class="src-flt-chip${active ? ' active' : ''}" data-src="${esc(s.name)}"` +
      ` role="button" tabindex="0" aria-pressed="${active}">` +
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

    // `theme` holding a URL is the legacy way of declaring a custom stylesheet.
    const cssUrl = data.custom_css || data.theme;
    if (stylesheetNeedsReload(cssUrl)) { location.reload(); return; }

    const { next, fresh: newAlerts } = diffKnown(App.knownFps, data);
    App.freshFps = new Set(newAlerts.map(a => a.fingerprint));
    if (newAlerts.length) {
      sendNotif(newAlerts);
      playSoundForAlerts(newAlerts);
    }
    App.knownFps = next;
    saveKnownFps(next);
    
    // Update config from API response
    if (data.timezone) AppConfig.timezone = data.timezone;
    applyCustomTheme(cssUrl);
    if (data.theme && THEME_PREFS.includes(data.theme) && !App.themeFromUser && !App.themeFromUrl) {
      applyTheme(data.theme, { persist: false });
    }
    if (data.play_sounds !== undefined) AppConfig.playSounds = data.play_sounds;
    
    App.data = data;
    applyTvDefault(data);

    render();

    App.lastSuccess = new Date();
    const now = App.lastSuccess.toLocaleTimeString('en-US');
    document.getElementById('last-refresh').textContent = now;
    document.getElementById('tv-last').textContent = now;
    setStale(false);
    startCountdown(data.refresh_interval);

    App.refreshTimer = setTimeout(fetchAlerts, data.refresh_interval * 1000);
  } catch (err) {
    console.error('AlertView:', err);
    setStale(true, err);
    // The countdown was cleared on the way in; restart it on the retry delay
    // so the page keeps visibly ticking instead of freezing mid-number.
    startCountdown(15);
    App.refreshTimer = setTimeout(fetchAlerts, 15000);
  } finally {
    App.loading = false;
    document.getElementById('spinner').style.display = 'none';
  }
}


function paintCountdown() {
  document.getElementById('countdown').textContent = App.countdown;
  document.getElementById('tv-cd').textContent = App.countdown;
}

function startCountdown(seconds) {
  clearInterval(App.countdownTimer);
  App.countdown = seconds;
  paintCountdown();
  App.countdownTimer = setInterval(() => {
    App.countdown = Math.max(0, App.countdown - 1);
    paintCountdown();
  }, 1000);
}

/* Shown until the next successful reload, or until dismissed. The reason comes
   from the server, which has already redacted any credentials in it. */
function showConfigError(reason) {
  const banner = document.getElementById('config-banner');
  if (!banner) return;
  banner.hidden = !reason;
  if (reason) document.getElementById('config-reason').textContent = String(reason).split('\n')[0];
}

/* A failed poll used to be a console message and nothing else: "last refresh"
   kept showing an old time and the countdown froze, so a dead backend looked
   exactly like "nothing new" — the worst way for a wall display to fail. */
function setStale(stale, err) {
  if (stale === App.stale && !stale) return;
  App.stale = stale;
  document.documentElement.setAttribute('data-stale', String(stale));
  const banner = document.getElementById('stale-banner');
  banner.hidden = !stale;
  if (!stale) { updateTitle(); return; }

  document.getElementById('stale-since').textContent = App.lastSuccess
    ? App.lastSuccess.toLocaleTimeString('en-US')
    : 'never';
  banner.title = String(err?.message || err || 'The alertview backend did not answer');
  document.title = '⚠ stale — AlertView';
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
/* Whether one alert survives the suppressed-alert selection. An empty selection
   is no filter at all. `firing` covers everything that is not suppressed, so a
   pending alert rides with it rather than needing a chip of its own. */
function statusAllowed(status) {
  if (App.statusFilter.size === 0) return true;
  const suppressed = status === 'silenced' || status === 'inhibited';
  return suppressed ? App.statusFilter.has(status) : App.statusFilter.has('firing');
}

function filteredAlerts() {
  const { filters, text } = parseQuery(App.searchQ || '');
  return (App.data?.alerts ?? []).filter(a => {
    if (!statusAllowed(a.status)) return false;
    // `|| 'none'` matches how the chips are counted: an alert whose source sent
    // no severity is counted under `none`, so the `none` chip has to find it.
    if (App.sevFilter.size > 0 && !App.sevFilter.has(a.severity || 'none')) return false;
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
  applyChipSelection(App.sevFilter, s);
  lsSet('av-sev-filter', formatFilterSet(App.sevFilter));
  renderStats();
  renderAlerts();
  TV.renderChips();
  pushUrl();
}

/* -- Render -- */
/* The tab title says the same thing as the toolbar counter: how many alerts
   are on screen, out of how many arrived. It used to count the firing ones
   whatever the filters said, so narrowing 54 alerts down to 3 with a search
   still read "54 alerts" in the tab — and it was only refreshed after a poll,
   never when a filter changed. Called from renderAlerts() now, which is the
   one function every filter goes through.
   `shown` and `total` are passed in by the caller that already computed them;
   setStale() calls it without arguments. */
function updateTitle(shown, total) {
  if (App.stale) { document.title = '⚠ stale — AlertView'; return; }
  shown ??= filteredAlerts();
  total ??= App.data?.alerts.length ?? 0;
  if (!total) { document.title = 'AlertView'; return; }

  const alertWord = total !== 1 ? 'alerts' : 'alert';
  const count = shown.length < total ? `${shown.length} / ${total}` : String(total);
  // Nothing left after filtering is not "all clear": say so with the glass the
  // empty state uses, not with the colour of a severity nothing is showing.
  const icon = shown.length ? severityIcon(shown) : '🔍';
  document.title = `${icon} ${count} ${alertWord} — AlertView`;
}

function render() { renderStats(); renderSources(); renderSourceChips(); renderAlerts(); TV.renderChips(); TV.renderDots(); renderStatusChips(); }

/* How many alerts of each severity, most severe first. */
function severityCounts() {
  const counts = {};
  (App.data?.alerts ?? []).forEach(a => {
    const s = a.severity || 'none';
    counts[s] = (counts[s] || 0) + 1;
  });
  return Object.keys(counts)
    .sort((a, b) => severityOrder(a) - severityOrder(b))
    .map(s => [s, counts[s]]);
}

/* One severity chip. The header and the TV panel build the same row and had
   drifted into two copies; `style` is the only thing that ever differed. */
function sevChipHtml(sev, label, style = '') {
  const active = sev === 'all' ? App.sevFilter.size === 0 : App.sevFilter.has(sev);
  const cls = sev === 'all' ? 'stat-chip' : `stat-chip ${sevClass(sev)}`;
  const title = sev === 'all' ? 'Every severity' : sev;
  return `<span class="${cls}${active ? ' active' : ''}"${style} data-sev="${esc(sev)}"` +
    ` role="button" tabindex="0" aria-pressed="${active}" title="${esc(title)}">${label}</span>`;
}

/* The `all` chip leads the row here as it does in the TV panel. The header used
   to omit it, which was survivable while one click could only ever select one
   severity; with several selected there has to be one thing that clears them. */
function renderStats() {
  document.getElementById('stats-bar').innerHTML =
    sevChipHtml('all', 'all') +
    severityCounts().map(([s, n]) => sevChipHtml(s, `${n}&thinsp;${esc(s)}`)).join('');
}

function renderSources() {
  document.getElementById('sources-bar').innerHTML = (App.data?.sources ?? []).map((s, i) => `
    ${i > 0 ? '<span class="src-sep">·</span>' : ''}
    <span class="src-item">
      <span class="src-dot ${s.status}"></span>
      ${esc(s.name)}
      ${s.status === 'ok' ? '· ' + s.alert_count + ' alert' + (s.alert_count !== 1 ? 's' : '')
        : s.status === 'pending' ? '<span class="src-pending-label">· waiting…</span>'
        : `<span class="src-err-label" title="${esc(s.error)}">⚠ error</span>`}
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

/* An empty list is only "all clear" when every source has answered. A source
   still pending (just after startup) or failing may be hiding alerts, and a
   green tick on a wall display would say otherwise. */
function emptyStateHtml() {
  /* A selection of suppressed kinds alone asks "what is being masked?", so an
     empty list is an answer — "nothing is silenced" — not the all-clear of an
     unfiltered view. Named in chip order so the sentence does not depend on
     which chip was clicked first. */
  const supp = STATUS_KINDS.filter(k => k !== 'firing' && App.statusFilter.has(k));
  const onlySuppressed = supp.length > 0 && !App.statusFilter.has('firing');
  const filtering = App.searchQ || App.sevFilter.size > 0 || App.srcFilter.size > 0
    || onlySuppressed;
  const sources = App.data?.sources ?? [];
  const pending = sources.filter(s => s.status === 'pending').map(s => s.name);
  const failing = sources.filter(s => s.status === 'error').map(s => s.name);
  let icon = '✅', text = 'No active alerts';
  if (filtering) {
    icon = '🔍';
    if (App.searchQ) text = 'No results for &laquo;&nbsp;' + esc(App.searchQ) + '&nbsp;&raquo;';
    else if (onlySuppressed) text = 'No ' + esc(supp.join(' or ')) + ' alerts';
  } else if (pending.length) {
    icon = '⏳';
    text = 'Waiting for ' + esc(pending.join(', '));
  } else if (failing.length) {
    icon = '⚠';
    text = 'No alerts from the sources that answered — unreachable: ' + esc(failing.join(', '));
  }
  return `<div class="empty-state">
      <div class="empty-state-icon">${icon}</div>
      <div>${text}</div>
    </div>`;
}

function renderAlerts() {
  const filtered = filteredAlerts();
  const total    = App.data?.alerts.length ?? 0;
  const listEl   = document.getElementById('alert-list');

  const alertWord = total !== 1 ? 'alerts' : 'alert';
  document.getElementById('alert-count').textContent = filtered.length < total
    ? filtered.length + ' / ' + total + ' ' + alertWord
    : total + ' ' + alertWord;

  // Same numbers in the tab, from the same two values.
  updateTitle(filtered, total);
  // And the same story in the TV HUD, which is all a wall screen has.
  TV.renderQuery();

  if (!filtered.length) {
    listEl.innerHTML = emptyStateHtml();
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
      <div class="group-header" role="button" tabindex="0" aria-expanded="false">
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
  groupEl.querySelector('.group-header').setAttribute('aria-expanded', String(isOpen));
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

/* `rel` regardless of the target: in kiosk mode the link opens in the same tab,
   and without it the dashboard URL — filters included — travelled to whatever
   the alert pointed at. */
function linkTarget() {
  const target = App.data?.link_new_tab === false ? '' : ' target="_blank"';
  return `${target} rel="noopener noreferrer"`;
}

/* The server only ever hands out http(s) links — `sanitize_link` in alerts.rs
   drops anything else. This is the same check on the way out, so the page does
   not depend on that promise holding for ever. */
function safeHref(url) {
  const s = String(url ?? '').trim();
  return /^https?:\/\//i.test(s) ? s : '';
}

/* The alert link hangs off the severity marker rather than the whole card: one
   small, deliberate target that says what it does on hover, instead of a row
   that navigates wherever you happen to click. */
function severityMarkLink(a, mark) {
  if (!a.alert_link_url) return mark;
  const href = safeHref(a.alert_link_url);
  if (!href) return mark;
  return `<a class="mark-link" href="${esc(href)}"${linkTarget()}` +
    ` title="${esc(a.name)} — open the runbook">${mark}</a>`;
}

function genLinkHtml(url, sourceType, sourceName) {
  url = safeHref(url);
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

  // The alert name is not a chip; when an annotation takes its place it would be
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

/* The main text of an alert: its name, or — when the config hides the name —
   the first of display.title_annotations the alert carries. An alert with none
   of them keeps its name rather than showing nothing. Returns the text and the
   annotation it consumed, if any, so it is not repeated below. */
function alertTitle(a) {
  if (App.data?.show_alert_name === false) {
    for (const key of App.data.title_annotations ?? ['summary']) {
      const text = a.annotations?.[key];
      if (text) return { text, annotation: key };
    }
  }
  return { text: a.name, annotation: null };
}

/* The emoji artwork itself, shipped with the app instead of borrowed from the
   machine displaying it. A typed emoji only appears if that machine has a
   colour emoji font: a minimal Linux box, a kiosk browser or a Windows N
   edition renders 🔥 as an empty box — on exactly the screen that most needs
   to be readable. These names resolve to Noto Color Emoji drawings embedded in
   the stylesheet, so the dashboard looks the same everywhere. Any other string
   in the config is still rendered as text. */
const BUILTIN_ICONS = {
  flame: '<span class="ic ic-flame"></span>',
  'bell-off': '<span class="ic ic-bell-off"></span>',
  hourglass: '<span class="ic ic-hourglass"></span>',
};

/* A built-in name draws its icon; anything else is text from the config and is
   escaped. hasOwn keeps a name like "constructor" from reaching Object's
   prototype and stringifying a function into the page. */
function iconHtml(value) {
  return Object.hasOwn(BUILTIN_ICONS, value) ? BUILTIN_ICONS[value] : esc(value);
}

function criticalIcon(a) {
  const icon = App.data?.critical_icon;
  if (!icon || canonSev(a.severity) !== 'critical') return '';
  return `<span class="crit-icon" aria-hidden="true">${iconHtml(icon)}</span>`;
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
  return `<span class="status-icon" title="${esc(a.status)}">${iconHtml(icon)}</span>`;
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
  const summary  = title.annotation === 'summary' ? '' : (a.annotations?.summary || '');
  const desc     = title.annotation === 'description' ? '' : (a.annotations?.description || '');
  const showDesc = desc && desc !== title.text && desc !== summary;

  return `
    <div class="alert-card ${sevClass(sev)}" data-fp="${esc(a.fingerprint)}">
      <div class="card-top">
        <div class="card-title">
          ${severityMark(a)}
          ${prefixHtml(a)}
          <span class="sev-badge ${sevClass(sev)}">${esc(sev)}</span>
          ${statusMark(a)}${commentToggleHtml(a)}
          <span class="alert-name${title.annotation ? ' is-summary' : ''}">${esc(title.text)}</span>
        </div>
        <div class="card-meta">
          <span class="src-chip">${esc(a.source)}</span>
          <span class="time-ago" title="${esc(absTime(a.starts_at))}">for&nbsp;${relTime(a.starts_at)}</span>
          ${genLinkHtml(a.link_url, a.source_type, a.source)}
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
  // A row has one line: once an annotation is the title, a second sentence next
  // to it is noise. The slot itself stays, empty — see the subgrid note below.
  const summary = title.annotation ? '' : (a.annotations?.summary || '');
  
  // A row only has space for 2 labels inline, the rest go behind the +N toggle.
  // Presence is filtered *before* slicing so a row never hides every label it
  // has; labels already shown in the prefix are excluded by chipLabels().
  const lay = labelLayout(a, 2);
  const labelsHtml = lay.visible.map(i => labelChip(i)).join('');

  // Every slot is always emitted, even empty: the row is a subgrid of the list,
  // so a missing element would shift the columns of that row only. This is what
  // lines the rows up — see the "TV mode: rows share one grid" block in the CSS.
  return `
    <div class="alert-card alert-row ${sevClass(sev)}" data-fp="${esc(a.fingerprint)}">
      <span class="row-lead">${severityMark(a)}</span>
      ${prefixHtml(a) || '<span class="alert-prefix"></span>'}
      <span class="sev-badge ${sevClass(sev)}">${esc(sev)}</span>
      <span class="row-status">${statusMark(a)}${commentToggleHtml(a)}</span>
      <span class="alert-name${title.annotation ? ' is-summary' : ''}">${esc(title.text)}</span>
      <span class="row-summary">${esc(summary)}</span>
      <span class="row-labels">${labelsHtml}${labelsToggleHtml(lay.hidden, lay.open)}</span>
      <span class="time-ago" title="${esc(absTime(a.starts_at))}">for&nbsp;${relTime(a.starts_at)}</span>
      <span class="row-link">${genLinkHtml(a.link_url, a.source_type, a.source)}</span>
      ${hiddenLabelsHtml(lay.hidden, lay.open)}
      ${commentHtml(a)}
    </div>`;
}

/* -- TV Mode -- */
const TV = {
  active:     false,
  /// Whether this browser has ever used the TV button. Declared here rather
  /// than sprouted in init(), so the shape of the object is the whole shape.
  chosen:     false,
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
    const options = { hour: '2-digit', minute: '2-digit', second: '2-digit', ...tzOptions() };
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
    const small = ' style="font-size:10px;padding:1px 7px"';
    document.getElementById('tv-sev-chips').innerHTML =
      sevChipHtml('all', 'all', small) +
      severityCounts().map(([s, n]) => sevChipHtml(s, `${n}&thinsp;${esc(s)}`, small)).join('');
  },

  renderDots() {
    document.getElementById('tv-dots').innerHTML = (App.data?.sources ?? [])
      .map(s => `<span class="src-dot ${s.status}" title="${esc(s.name)}${s.error ? ': ' + esc(s.error) : ''}"></span>`)
      .join('');
  },

  /* The active search, in the half of the HUD that never hides. The panel closes
     on a click anywhere else and the tab title is invisible full screen, so
     without this a wall can show a fraction of the alerts and say nothing. */
  renderQuery() {
    const el = document.getElementById('tv-q');
    if (!el) return;
    el.hidden = !App.searchQ;
    // Emptied as well as hidden: leaving the text behind is how a cleared search
    // kept showing its old query.
    el.innerHTML = App.searchQ ? '&#128269;&thinsp;' + esc(App.searchQ) : '';
    el.title = App.searchQ ? 'Search active: ' + App.searchQ : '';
  },
};

TV.init();

/* -- URL state sync -- */
function pushUrl() {
  const p = new URLSearchParams();
  if (App.themePref !== 'auto') p.set('theme', App.themePref);
  if (App.sevFilter.size > 0)  p.set('sev', [...App.sevFilter].join(','));
  if (App.srcFilter.size > 0)  p.set('src', [...App.srcFilter].join(','));
  if (App.searchQ)             p.set('q',        App.searchQ);
  // The default is "firing alone", so that one needs no parameter; every other
  // selection does, including the empty one that means "show everything".
  if (!(App.statusFilter.size === 1 && App.statusFilter.has('firing'))) {
    p.set('show', formatFilterSet(App.statusFilter));
  }
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
  if (p.has('sev'))   App.sevFilter = parseFilterSet(p.get('sev'));
  if (p.has('src'))   App.srcFilter = new Set(p.get('src').split(',').filter(Boolean));
  if (p.has('q')) {
    App.searchQ = p.get('q');
    syncSearchInputs();
  }
  // `?silenced=1` still works: it is on wall screens and in bookmarks already.
  if (p.has('silenced')) {
    App.statusFilter = new Set(p.get('silenced') === '1' ? [] : ['firing']);
  }
  if (p.has('show')) {
    const wanted = [...parseFilterSet(p.get('show'))].filter(m => STATUS_KINDS.includes(m));
    // An unknown name would otherwise empty the set and show everything, which
    // is the opposite of what a narrowing parameter asks for.
    if (wanted.length || p.get('show') === 'all') App.statusFilter = new Set(wanted);
  }
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

/* The chips and the group headers are spans, not <button>s, so Enter and Space
   have to be wired by hand — without this the whole filter row and every group
   header were unreachable from the keyboard. */
function activate(containerId, selector, handler) {
  delegate(containerId, selector, handler);
  const el = document.getElementById(containerId);
  if (!el) return;
  el.addEventListener('keydown', e => {
    if (e.key !== 'Enter' && e.key !== ' ' && e.key !== 'Spacebar') return;
    const hit = e.target.closest(selector);
    if (!hit || !el.contains(hit)) return;
    e.preventDefault();
    handler(hit, e);
  });
}

['stats-bar', 'tv-sev-chips'].forEach(id =>
  activate(id, '[data-sev]', el => toggleSev(el.dataset.sev)));
['src-filter-chips', 'tv-src-chips'].forEach(id =>
  activate(id, '[data-src]', el => toggleSrc(el.dataset.src)));
['status-filter-chips', 'tv-status-chips'].forEach(id =>
  activate(id, '[data-status]', el => setStatusFilter(el.dataset.status)));
/* Both row toggles do the same thing to a different set, keyed by fingerprint
   so the open state survives the refresh. */
function toggleOnCard(set) {
  return (el, e) => {
    // The severity marker next to these is a link; stop the click reaching it.
    e.preventDefault();
    e.stopPropagation();
    const fp = el.closest('.alert-card')?.dataset.fp;
    if (!fp) return;
    if (set.has(fp)) set.delete(fp);
    else set.add(fp);
    renderAlerts();
  };
}
delegate('alert-list', '[data-comment-toggle]', toggleOnCard(App.openComments));
delegate('alert-list', '[data-labels-toggle]', toggleOnCard(App.openLabels));
activate('alert-list', '.group-header', el => {
  const groupEl = el.closest('.alert-group');
  if (groupEl) toggleGroup(groupEl.dataset.groupKey, groupEl);
});

/* Ctrl+F / Cmd+F and "/" mean "let me search" — a named function because the
   TV-mode case is the part worth pinning in a test. */
function wantsSearchFocus(e) {
  const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(e.target.tagName) || e.target.isContentEditable;
  if (e.key === 'f' || e.key === 'F') return !!(e.ctrlKey || e.metaKey);
  return e.key === '/' && !typing && !e.ctrlKey && !e.metaKey;
}

/* Both take over from the browser's find-in-page, which only ever finds what is
   already on screen — the wrong tool on a filtered list of hundreds. In TV mode
   this reveals the filter panel and lands in its search field; the header box is
   inside a hidden element there and cannot take focus. */
document.addEventListener('keydown', e => {
  if (!wantsSearchFocus(e)) return;
  e.preventDefault();
  let field = SearchInput;
  if (TV.active) {
    TV.openPanel();
    field = document.getElementById('tv-search');
  }
  field.focus();
  field.select();
});

/* Escape leaves the search box, clearing it when it is empty of intent. In TV
   mode it then closes the panel — blur() first, so the next Escape reaches the
   document handler and carries on down the chain (panel → + → leave TV). */
SearchInputs.forEach(el => el.addEventListener('keydown', e => {
  if (e.key !== 'Escape') return;
  if (App.searchQ) { clearSearch(); return; }
  el.blur();
  if (TV.active) TV.closePanel();
}));

/* -- Boot -- */
// The <head> script already resolved the theme to avoid a flash; this syncs the
// button icons and the theme-color meta with it.
applyTheme(App.themePref, { persist: false });
initFromUrl();
renderStatusChips();
fetchAlerts();

/* Register the service worker so AlertView is installable as a PWA. Lives here
   rather than in an inline <script> so the page needs no `unsafe-inline`. */
if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch(err => {
      console.warn('Service worker registration failed:', err);
    });
  });
}
