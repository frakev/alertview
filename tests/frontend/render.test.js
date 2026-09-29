// Runs the real render functions from static/app.js against alert data shaped
// like the API's, and asserts the properties that have broken before.
//
// Run with: node tests/frontend/render.test.js
const assert = require('assert');
const { load } = require('./extract.js');

const CONSTS = ['ESC', 'DEFAULT_SEV_ORDER', 'SEV_ALIASES', 'MISSING_LABEL',
                'sevOrderCache', 'BUILTIN_ICONS', 'SOUND_PRESETS', 'tzChecked', 'THEME_PREFS'];
const FNS = ['esc', 'sevClass', 'canonSev', 'sevOrderList', 'severityOrder', 'severityIcon',
             'presetFor', 'relTime', 'absTime', 'tzOptions', 'linkTarget', 'getSourceLabel',
             'genLinkHtml', 'severityMarkLink', 'safeHref', 'iconHtml', 'prefixLabels', 'prefixHtml',
             'chipLabels', 'labelLayout', 'labelChip', 'hiddenLabelsHtml', 'labelsToggleHtml',
             'alertTitle', 'criticalIcon', 'severityMark', 'statusMark', 'alertComment',
             'commentAuthor', 'commentToggleHtml', 'commentHtml', 'cardHtml', 'cardHtmlTV',
             'alertsInGroup', 'parseQuery', 'alertField', 'matchFilter', 'filtersMatch',
             'diffKnown', 'emptyStateHtml', 'cssOrigin', 'stylesheetNeedsReload'];

const App = { data: null, openLabels: new Set(), openComments: new Set(),
              searchQ: '', sevFilter: 'all', srcFilter: new Set() };
const location = { href: 'https://alertview.test/', origin: 'https://alertview.test' };
const AppConfig = { timezone: 'local', playSounds: true };
const TV = { active: false };
const H = load(FNS, CONSTS, { App, AppConfig, TV, location });

let checks = 0;
const ok = (cond, msg) => { assert(cond, msg); checks++; };

/* The attribute names a browser would actually see. Quoted values are skipped
   the way the HTML tokeniser skips them, so escaped text that merely reads like
   `onmouseover=` inside a title is not mistaken for an attribute. */
function attributeNames(html) {
  const names = [];
  for (let i = 0; i < html.length; i++) {
    if (html[i] !== '<' || !/[a-z]/i.test(html[i + 1] || '')) continue;
    i += 1;
    while (i < html.length && /[^\s/>]/.test(html[i])) i++;
    while (i < html.length && html[i] !== '>') {
      while (i < html.length && /[\s/]/.test(html[i])) i++;
      let name = '';
      while (i < html.length && /[^\s=/>]/.test(html[i])) name += html[i++];
      if (name) names.push(name.toLowerCase());
      while (i < html.length && /\s/.test(html[i])) i++;
      if (html[i] === '=') {
        i++;
        while (i < html.length && /\s/.test(html[i])) i++;
        const q = html[i];
        if (q === '"' || q === "'") { i++; while (i < html.length && html[i] !== q) i++; i++; }
        else while (i < html.length && !/[\s>]/.test(html[i])) i++;
      }
    }
  }
  return names;
}
const executable = html =>
  attributeNames(html).filter(n => n.startsWith('on') || n === 'autofocus');

App.data = {
  severity_order: ['critical', 'error', 'high', 'warning', 'info', 'none'],
  display_labels: ['team', 'host', 'namespace'],
  prefix_labels: ['hostname'],
  prefix_separator: ' / ',
  show_alert_name: true,
  show_labels: true,
  critical_icon: 'flame',
  status_icons: { silenced: 'bell-off', pending: 'hourglass' },
  link_new_tab: true,
  groups: [],
};

// ── Escaping ────────────────────────────────────────────────────────────
// A label holding a double quote used to close the attribute it sat in and
// turn the next word into a live event handler.
const XSS = '" onmouseover="steal()" autofocus x="';
ok(!H.esc(XSS).includes('"'), 'esc must escape double quotes: ' + H.esc(XSS));
ok(H.esc("'").includes('&#39;'), 'esc must escape single quotes');

const hostile = {
  fingerprint: 'fp:' + XSS, source: XSS, source_type: 'alertmanager',
  status: 'silenced', severity: XSS, name: XSS,
  labels: { hostname: XSS, team: XSS, acknowledged_by: XSS },
  annotations: { summary: XSS, description: XSS, silence_comment: XSS, silence_created_by: XSS },
  starts_at: '2026-09-11T10:00:00Z', link_url: 'https://src.test/a',
  alert_link_url: 'https://wiki.test/a',
};
for (const open of [false, true]) {
  if (open) { App.openLabels.add(hostile.fingerprint); App.openComments.add(hostile.fingerprint); }
  TV.active = false;
  ok(executable(H.cardHtml(hostile)).length === 0, 'card: ' + executable(H.cardHtml(hostile)));
  TV.active = true;
  ok(executable(H.cardHtmlTV(hostile)).length === 0, 'row: ' + executable(H.cardHtmlTV(hostile)));
}
TV.active = false;
App.openLabels.clear(); App.openComments.clear();

// ── The TV row is a subgrid: every slot is always emitted ────────────────
// A missing element shifts that row's columns out of line with the rest.
const SLOTS = ['row-lead', 'alert-prefix', 'sev-badge', 'row-status', 'alert-name',
               'row-summary', 'row-labels', 'time-ago', 'row-link'];
const plain = {
  fingerprint: 'p1', source: 'AM', source_type: 'alertmanager', status: 'firing',
  severity: 'warning', name: 'DiskFull', labels: {}, annotations: {},
  starts_at: '2026-09-11T10:00:00Z', link_url: null, alert_link_url: null,
};
TV.active = true;
const row = H.cardHtmlTV(plain);
let at = -1;
for (const slot of SLOTS) {
  const found = row.indexOf(slot, at + 1);
  ok(found > at, `TV row: slot ${slot} missing or out of order`);
  at = found;
}
TV.active = false;

// ── Icons ───────────────────────────────────────────────────────────────
ok(H.severityMark({ ...plain, severity: 'critical' }).includes('ic-flame'), 'critical gets the flame');
ok(H.severityMark(plain).includes('sev-dot'), 'other severities keep the dot');
ok(H.statusMark({ ...plain, status: 'pending' }).includes('ic-hourglass'), 'pending gets the hourglass');
ok(H.statusMark(plain) === '', 'firing is the norm and gets no marker');
// A config string is still rendered as text, and still escaped.
App.data.critical_icon = '<img src=x onerror=1>';
ok(!H.criticalIcon({ ...plain, severity: 'critical' }).includes('<img'), 'config text is escaped');
App.data.critical_icon = '';
ok(H.severityMark({ ...plain, severity: 'critical' }).includes('sev-dot'), 'empty icon restores the dot');
App.data.critical_icon = 'flame';

// ── Links ───────────────────────────────────────────────────────────────
// The server drops anything that is not http(s); this is the same check on the
// way out, so the page does not lean on that promise holding for ever.
ok(H.safeHref('https://wiki.test/a') === 'https://wiki.test/a');
ok(H.safeHref('javascript:alert(1)') === '', 'javascript: must not become an href');
ok(H.safeHref('data:text/html,x') === '', 'data: must not become an href');
ok(H.safeHref(null) === '' && H.safeHref(undefined) === '');
ok(H.severityMark({ ...plain, alert_link_url: 'javascript:alert(1)' }).indexOf('<a') === -1,
   'a rejected URL leaves the marker unlinked rather than linking nowhere');
// rel travels even in kiosk mode, where the link opens in the same tab.
App.data.link_new_tab = false;
ok(H.linkTarget().includes('rel="noopener noreferrer"'), 'kiosk links keep their rel');
ok(!H.linkTarget().includes('target='), 'kiosk links stay in the tab');
App.data.link_new_tab = true;

// ── Severity ordering drives the icons and the sounds ───────────────────
const sev = s => ({ severity: s });
ok(H.severityIcon([sev('critical')]) === '🔴');
ok(H.severityIcon([sev('error')]) === '🟠');
ok(H.severityIcon([sev('warning')]) === '🟡');
ok(H.severityIcon([sev('warning'), sev('critical')]) === '🔴', 'the worst wins');
App.data.severity_order = ['disaster', 'major', 'minor'];
H.sevOrderList();
ok(H.presetFor('disaster') !== null, 'a renamed severity is never silent');
App.data.severity_order = ['critical', 'error', 'high', 'warning', 'info', 'none'];

// ── Label filters ───────────────────────────────────────────────────────
const q = H.parseQuery('team=sre|dba, hostname~web, disk full');
ok(q.filters.length === 2 && q.text === 'disk full', 'filters split from free text');
const a = { name: 'X', severity: 'warning', status: 'firing', source: 'AM',
            labels: { team: 'dba', hostname: 'web-01' }, annotations: {} };
ok(H.filtersMatch(a, H.parseQuery('team=sre|dba').filters), 'OR across values');
ok(!H.filtersMatch(a, H.parseQuery('team=sre').filters), 'and it still narrows');
ok(H.filtersMatch(a, H.parseQuery('team!=sre').filters), 'negation');

// ── Title annotation ────────────────────────────────────────────────────
// With the name hidden, the first configured annotation the alert carries
// becomes the title and is not repeated below it.
const annotated = { ...plain, annotations: { summary: 'SUM', description: 'DESC' } };
App.data.show_alert_name = false;
App.data.title_annotations = ['description', 'summary'];
ok(H.alertTitle(annotated).text === 'DESC', 'first configured annotation wins');
ok(H.alertTitle({ ...plain, annotations: { summary: 'SUM' } }).text === 'SUM', 'falls back down the list');
ok(H.alertTitle(plain).text === 'DiskFull', 'no annotation keeps the name');
const card = H.cardHtml(annotated);
ok(card.split('DESC').length === 2 && card.includes('card-summary'), 'description not repeated, summary kept');
delete App.data.title_annotations;
ok(H.alertTitle(annotated).text === 'SUM', 'summary by default');
App.data.show_alert_name = true;
ok(H.alertTitle(annotated).text === 'DiskFull', 'name shown unless hidden');

// ── New alerts are tracked per source ──────────────────────────────────
// A response missing a source — pending at startup, or failing — used to make
// that source's whole backlog "new" once it came back: sounds and notifications
// for every alert already firing.
const al = (fp, source) => ({ fingerprint: fp, source });
const resp = (alerts, sources) => ({ alerts, sources });
let r = H.diffKnown(null, resp([al('a', 'A')], [{ name: 'A', status: 'ok' }, { name: 'B', status: 'pending' }]));
ok(r.fresh.length === 0, 'nothing is new on the first response');
r = H.diffKnown(r.next, resp([al('a', 'A'), al('b1', 'B'), al('b2', 'B')],
  [{ name: 'A', status: 'ok' }, { name: 'B', status: 'ok' }]));
ok(r.fresh.length === 0, "a source answering late is primed, not announced");
r = H.diffKnown(r.next, resp([al('a', 'A')], [{ name: 'A', status: 'ok' }, { name: 'B', status: 'error' }]));
r = H.diffKnown(r.next, resp([al('a', 'A'), al('b1', 'B'), al('b2', 'B'), al('b3', 'B')],
  [{ name: 'A', status: 'ok' }, { name: 'B', status: 'ok' }]));
ok(r.fresh.length === 1 && r.fresh[0].fingerprint === 'b3', 'after an outage only the really new alert is announced');

// ── The empty list only says "all clear" when every source answered ────
App.data = { ...App.data, alerts: [], sources: [{ name: 'A', status: 'ok' }, { name: 'B', status: 'pending' }] };
ok(H.emptyStateHtml().includes('⏳') && !H.emptyStateHtml().includes('✅'), 'pending source: no green tick');
App.data.sources[1].status = 'error';
ok(H.emptyStateHtml().includes('⚠') && H.emptyStateHtml().includes('B'), 'failing source is named');
App.data.sources[1].status = 'ok';
ok(H.emptyStateHtml().includes('✅'), 'all answered and empty: all clear');
App.data.sources = [{ name: '<b>', status: 'pending' }];
ok(!H.emptyStateHtml().includes('<b>'), 'source names are escaped');

// ── A stylesheet on another host reloads the page ───────────────────────
ok(H.stylesheetNeedsReload('https://a.test/t.css') === false, 'the first response only records the host');
ok(H.stylesheetNeedsReload('https://a.test/other.css') === false, 'same host applies in place');
ok(H.stylesheetNeedsReload(undefined) === false, 'removing it applies in place');
ok(H.stylesheetNeedsReload('/local.css') === false, "a same-origin stylesheet is covered by 'self'");
ok(H.stylesheetNeedsReload('https://b.test/t.css') === true, 'another host needs the new policy');
ok(H.cssOrigin('dark') === null, 'a theme name is not a stylesheet');

// ── Timezone ────────────────────────────────────────────────────────────
AppConfig.timezone = 'Europe/Nowhere';
assert.doesNotThrow(() => new Date().toLocaleTimeString('en-US', { ...H.tzOptions() }),
  'an unknown timezone must not throw — it used to kill the TV clock every second');
AppConfig.timezone = 'local';

console.log(`static/app.js: ${checks} assertions passed`);
