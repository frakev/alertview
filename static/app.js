/* -- Utilities -- */
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

/* -- Theme -- */
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

// Sound presets by severity
const SOUND_PRESETS = {
  critical: () => { AudioContext.playBeep(800, 300); AudioContext.playBeep(600, 300); },
  high:     () => { AudioContext.playBeep(600, 200); AudioContext.playBeep(500, 200); },
  warning:  () => { AudioContext.playBeep(400, 150); },
  info:     () => { AudioContext.playBeep(300, 100); }
};

function applyTheme(t) {
  document.documentElement.setAttribute('data-theme', t);
  localStorage.setItem('av-theme', t);
  document.getElementById('theme-ico').innerHTML    = t === 'dark' ? MOON : SUN;
  document.getElementById('tv-theme-ico').innerHTML = t === 'dark' ? MOON : SUN;
}

// Apply custom theme CSS if provided
function applyCustomTheme(cssUrl) {
  const existing = document.getElementById('custom-theme-css');
  if (existing) existing.remove();
  
  if (cssUrl && cssUrl !== 'dark' && cssUrl !== 'light') {
    const link = document.createElement('link');
    link.id = 'custom-theme-css';
    link.rel = 'stylesheet';
    link.href = cssUrl;
    document.head.appendChild(link);
  }
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

/* -- knownFps persistence -- */
function loadKnownFps() {
  try {
    const raw = localStorage.getItem('av-known-fps');
    if (!raw) return null;
    const { fps, ts } = JSON.parse(raw);
    if (Date.now() - ts > 86400000) return null;
    return new Set(fps);
  } catch { return null; }
}

function saveKnownFps(fps) {
  try {
    localStorage.setItem('av-known-fps', JSON.stringify({ fps: [...fps], ts: Date.now() }));
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

  eventSource.addEventListener('new_alert', (event) => {
    try {
      const alert = JSON.parse(event.data);
      console.log('New alert received via SSE:', alert);
      
      // Play sound if enabled
      if (AppConfig.playSounds) {
        playSoundForAlerts([alert]);
      }
      
      // Show desktop notification if permitted
      if (Notification?.permission === 'granted') {
        sendNotif([alert]);
      }
      
      // Refresh the alerts list
      fetchAlerts();
    } catch (e) {
      console.error('Error processing SSE event:', e);
    }
  });

  // Reload config when it changes (e.g., display_labels)
  eventSource.addEventListener('config_reloaded', () => {
    console.log('Config reloaded via SSE, refreshing alerts...');
    fetchAlerts();
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
  const severities = ['critical', 'high', 'warning', 'info'];
  for (const sev of severities) {
    if (newAlerts.some(a => a.severity === sev)) {
      if (SOUND_PRESETS[sev]) {
        SOUND_PRESETS[sev]();
      }
      break; // Only play for the highest severity
    }
  }
}

function sendNotif(newAlerts) {
  if (Notification?.permission !== 'granted' || !newAlerts.length) return;
  const bySev = s => newAlerts.filter(a => a.severity === s).length;
  const icon = bySev('critical') ? '🔴' : bySev('high') ? '🟠' : '🟡';
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
  knownFps:       loadKnownFps(),
  freshFps:       new Set(),
  searchQ:        '',
  sevFilter:      localStorage.getItem('av-sev-filter') || 'all',
  srcFilter:      (() => { try { const r = localStorage.getItem('av-src-filter'); return new Set(r ? JSON.parse(r) : []); } catch { return new Set(); } })(),
  showSilenced:   localStorage.getItem('av-show-silenced') === 'true',
  refreshTimer:   null,
  countdownTimer: null,
  countdown:      0,
  loading:        false,
  openGroups:     new Set(),
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
  localStorage.setItem('av-show-silenced', App.showSilenced);
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
  localStorage.setItem('av-src-filter', JSON.stringify([...App.srcFilter]));
  renderSourceChips();
  renderAlerts();
  pushUrl();
}

function renderSourceChips() {
  const sources = App.data?.sources ?? [];
  const chips = sources.map(s => {
    const active = App.srcFilter.has(s.name);
    const count  = s.status === 'ok' ? `&thinsp;(${s.alert_count})` : '';
    return `<span class="src-flt-chip${active ? ' active' : ''}" onclick='toggleSrc(${JSON.stringify(s.name)})'>` +
      `<span class="src-dot ${s.status}"></span>${esc(s.name)}${count}</span>`;
  }).join('');
  ['src-filter-chips', 'tv-src-chips'].forEach(id => {
    const el = document.getElementById(id);
    if (el) el.innerHTML = chips;
  });
}

/* -- Refresh -- */
document.getElementById('refresh-btn').addEventListener('click', () => fetchAlerts());

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
    if (data.theme) applyCustomTheme(data.theme);
    if (data.play_sounds !== undefined) AppConfig.playSounds = data.play_sounds;
    
    App.data = data;

    render();

    document.getElementById('last-refresh').textContent = new Date().toLocaleTimeString('en-US');

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


/* -- Filters -- */
function filteredAlerts() {
  return (App.data?.alerts ?? []).filter(a => {
    if (!App.showSilenced && a.status === 'silenced') return false;
    if (App.sevFilter !== 'all' && a.severity !== App.sevFilter) return false;
    if (App.srcFilter.size > 0 && !App.srcFilter.has(a.source)) return false;
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

/* -- Render -- */
function updateTitle() {
  const firing = (App.data?.alerts ?? []).filter(a => a.status === 'firing');
  if (!firing.length) { document.title = 'AlertView'; return; }
  const bySev = s => firing.filter(a => a.severity === s).length;
  const icon = bySev('critical') ? '🔴' : bySev('high') ? '🟠' : '🟡';
  const alertWord = firing.length > 1 ? 'alerts' : 'alert';
  document.title = `${icon} ${firing.length} ${alertWord} — AlertView`;
}

function render() { renderStats(); renderSources(); renderSourceChips(); renderAlerts(); TV.renderChips(); TV.renderDots(); updateSilenceBtn(); updateTitle(); }

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
      ${s.status === 'ok' ? '· ' + s.alert_count + ' alert' + (s.alert_count !== 1 ? 's' : '') : `<span class="src-err-label" title="${esc(s.error)}">⚠ error</span>`}
    </span>`).join('');
}

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

function alertsInGroup(group, filtered) {
  const groupKeys = group.key.split(',').map(k => {
    const [key, value] = k.split('=');
    return { key, value };
  });
  return filtered.filter(a => groupKeys.every(({ key, value }) => a.labels?.[key] === value));
}

function groupSevBadges(group) {
  return Object.entries(group.severity_counts || {})
    .sort(([a], [b]) => severityOrder(a) - severityOrder(b))
    .map(([sev, count]) => `<span class="sev-badge sev-${sev}">${count} ${sev}</span>`)
    .join('');
}

/* Group shell (header + empty body); cards are reconciled separately so that
   expand/collapse state and individual cards survive a refresh. */
function groupShellHtml(group) {
  const groupLabel = group.key.split(',').map(k => {
    const [key, value] = k.split('=');
    return `<span class="lbl">${esc(key)}=<b>${esc(value)}</b></span>`;
  }).join('');

  return `
    <div class="alert-group" data-group-key="${esc(group.key)}">
      <div class="group-header" onclick="toggleGroup('${esc(group.key)}')">
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

function severityOrder(sev) {
  const order = { critical: 0, high: 1, warning: 2, warn: 2, info: 3, none: 4 };
  return order[sev] ?? 5;
}

function toggleGroup(groupKey) {
  if (App.openGroups.has(groupKey)) App.openGroups.delete(groupKey);
  else App.openGroups.add(groupKey);

  const inner = document.getElementById('group-' + groupKey);
  const groupEl = inner && inner.closest('.alert-group');
  if (groupEl) applyGroupOpen(groupEl, App.openGroups.has(groupKey));
}

function getSourceLabel(sourceType) {
  const labels = {
    alertmanager: "Open in Alertmanager",
    grafana: "Open in Grafana",
    zabbix: "Open in Zabbix"
  };
  return labels[sourceType] || "Open in source";
}

function genLinkHtml(url, sourceType) {
  if (!url) return '';
  const label = sourceType ? getSourceLabel(sourceType) : "Open in Prometheus/Grafana";
  return `<a href="${esc(url)}" target="_blank" rel="noopener noreferrer" class="gen-link" title="${esc(label)}">
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
  
  // Check for silence/acknowledgement comments
  const ackComment = a.annotations?.acknowledgement || a.annotations?.silence_comment || '';

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
          <span class="time-ago" title="${esc(absTime(a.starts_at))}">for&nbsp;${relTime(a.starts_at)}</span>
          ${genLinkHtml(a.link_url, a.source_type)}
        </div>
      </div>
      ${summary  ? `<div class="card-summary">${esc(summary)}</div>` : ''}
      ${showDesc ? `<div class="card-desc">${esc(desc)}</div>` : ''}
      ${ackComment ? `<div class="card-ack-comment"><strong>Comment:</strong> ${esc(ackComment)}</div>` : ''}
      ${labels   ? `<div class="label-chips">${labels}</div>` : ''}
    </div>`;
}

function cardHtmlTV(a) {
  const sev     = a.severity || 'none';
  const summary = a.annotations?.summary || '';
  const ackComment = a.annotations?.acknowledgement || a.annotations?.silence_comment || '';
  
  // Prepare labels for TV mode
  const display_labels = App.data?.display_labels ?? [];
  const visible_labels = display_labels.slice(0, 2); // Show first 2 labels by default
  const hidden_labels = display_labels.slice(2); // Rest are hidden
  const has_hidden_labels = hidden_labels.length > 0;
  
  // Generate label HTML for visible labels
  const labelsHtml = visible_labels
    .filter(l => a.labels?.[l] !== undefined && l !== 'alertname' && l !== 'severity')
    .map(l => `<span class="lbl tv-lbl">${esc(l)}=<b>${esc(a.labels[l])}</b></span>`)
    .join('');
  
  // Generate hidden labels HTML (initially hidden)
  const hiddenLabelsList = hidden_labels
    .filter(l => a.labels?.[l] !== undefined && l !== 'alertname' && l !== 'severity');
  const hiddenLabelsHtml = hiddenLabelsList.length > 0
    ? `<span class="tv-hidden-labels" style="display:none;">` +
      hiddenLabelsList.map(l => `<span class="lbl tv-lbl">${esc(l)}=<b>${esc(a.labels[l])}</b></span>`).join('') +
      `</span>` : '';
  const showToggle = hiddenLabelsList.length > 0;

  return `
    <div class="alert-card alert-row sev-${sev}" data-fp="${esc(a.fingerprint)}">
      <span class="sev-dot sev-${sev}"></span>
      <span class="alert-name">${esc(a.name)}</span>
      <span class="sev-badge sev-${sev}">${sev}</span>
      <span class="status-badge status-${a.status}">${a.status}</span>
      <span class="row-summary">${esc(summary)}</span>
      <span class="src-chip">${esc(a.source)}</span>
      <span class="time-ago" title="${esc(absTime(a.starts_at))}">for&nbsp;${relTime(a.starts_at)}</span>
      ${ackComment ? `<span class="tv-ack-comment">Comment: ${esc(ackComment)}</span>` : ''}
      ${labelsHtml}
      ${showToggle ? `<button class="tv-labels-toggle" onclick="TV.toggleLabels(this)">+${hiddenLabelsList.length}</button>` : ''}
      ${hiddenLabelsHtml}
      ${genLinkHtml(a.link_url, a.source_type)}
    </div>`;
}

/* -- TV Mode -- */
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

    // Show bar on any movement
    document.addEventListener('mousemove', () => this.showBar());
    document.addEventListener('click',     () => this.showBar());

    // Close panel when clicking elsewhere
    document.addEventListener('click', e => {
      if (this.panelOpen && !document.getElementById('tv-panel').contains(e.target) && e.target.id !== 'tv-settings-btn') {
        this.closePanel();
      }
    });

    // Keyboard shortcuts
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

  // Toggle labels visibility in TV mode
  toggleLabels(button) {
    const hiddenLabels = button.parentElement.querySelector('.tv-hidden-labels');
    if (hiddenLabels) {
      const isHidden = hiddenLabels.style.display === 'none';
      hiddenLabels.style.display = isHidden ? 'inline' : 'none';
      const count = hiddenLabels.querySelectorAll('.lbl').length;
      button.textContent = isHidden ? '−' : `+${count}`;
    }
  },

  _apply() {
    document.documentElement.setAttribute('data-tv', this.active);
    document.getElementById('tv-btn').classList.toggle('active', this.active);
    if (this.active) {
      this.startClock();
      this.showBar();
      this.renderChips();
      this.renderDots();
      renderSourceChips();
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
    const options = { hour: '2-digit', minute: '2-digit', second: '2-digit' };
    if (AppConfig.timezone !== 'local') {
      options.timeZone = AppConfig.timezone;
    }
    document.getElementById('tv-clock').textContent =
      new Date().toLocaleTimeString('en-US', options);
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
    const all = `<span class="stat-chip${App.sevFilter === 'all' ? ' active' : ''}" style="font-size:10px;padding:1px 7px" onclick="toggleSev('all')">all</span>`;
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

/* -- URL state sync -- */
function pushUrl() {
  const p = new URLSearchParams();
  const theme = document.documentElement.getAttribute('data-theme');
  if (theme !== 'dark')        p.set('theme', theme);
  if (App.sevFilter !== 'all') p.set('sev',      App.sevFilter);
  if (App.srcFilter.size > 0)  p.set('src', [...App.srcFilter].join(','));
  if (App.searchQ)             p.set('q',        App.searchQ);
  if (App.showSilenced)        p.set('silenced', '1');
  const qs = p.toString();
  history.replaceState(null, '', qs ? '?' + qs : location.pathname);
}

function initFromUrl() {
  const p = new URLSearchParams(location.search);
  if (p.has('theme')) applyTheme(p.get('theme'));
  if (p.has('sev'))   { App.sevFilter = p.get('sev');   localStorage.setItem('av-sev-filter', App.sevFilter); }
  if (p.has('src'))   { const s = p.get('src').split(',').filter(Boolean); App.srcFilter = new Set(s); localStorage.setItem('av-src-filter', JSON.stringify(s)); }
  if (p.has('q'))        {
    App.searchQ = p.get('q');
    SearchInput.value = App.searchQ;
    SearchClear.style.display = App.searchQ ? 'block' : 'none';
  }
  if (p.has('silenced')) { App.showSilenced = p.get('silenced') === '1'; localStorage.setItem('av-show-silenced', App.showSilenced); }
}

/* -- Boot -- */
initFromUrl();
updateSilenceBtn();
fetchAlerts();
