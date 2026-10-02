// Runs the real render functions from static/app.js against alert data shaped
// like the API's, and asserts the properties that have broken before.
//
// Run with: node tests/frontend/render.test.js
const assert = require('assert');
const { load } = require('./extract.js');

const CONSTS = ['ESC', 'DEFAULT_SEV_ORDER', 'SEV_ALIASES', 'MISSING_LABEL',
                'sevOrderCache', 'BUILTIN_ICONS', 'SOUND_PRESETS', 'tzChecked', 'THEME_PREFS',
                'STATUS_CHIPS', 'STATUS_KINDS'];
const FNS = ['esc', 'sevClass', 'canonSev', 'sevOrderList', 'severityOrder', 'severityIcon',
             'presetFor', 'relTime', 'absTime', 'tzOptions', 'linkTarget', 'getSourceLabel',
             'genLinkHtml', 'severityMarkLink', 'safeHref', 'iconHtml', 'prefixLabels', 'prefixHtml',
             'chipLabels', 'labelLayout', 'labelChip', 'hiddenLabelsHtml', 'labelsToggleHtml',
             'alertTitle', 'criticalIcon', 'severityMark', 'statusMark', 'alertComment',
             'commentAuthor', 'commentToggleHtml', 'commentHtml', 'cardHtml', 'cardHtmlTV',
             'alertsInGroup', 'parseQuery', 'alertField', 'matchFilter', 'filtersMatch',
             'diffKnown', 'emptyStateHtml', 'cssOrigin', 'stylesheetNeedsReload',
             'wantsSearchFocus', 'searchTarget', 'wantsBrowserFind',
             'statusAllowed', 'filteredAlerts',
             'applyChipSelection', 'parseFilterSet', 'formatFilterSet', 'severityCounts'];

const App = { data: null, openLabels: new Set(), openComments: new Set(),
              searchQ: '', sevFilter: new Set(), srcFilter: new Set(),
              statusFilter: new Set(['firing']) };
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
TV.active = true;
const tvRow = H.cardHtmlTV(annotated);
ok(tvRow.includes('DESC') && !tvRow.includes('SUM'), 'TV row: an annotation as title, no summary beside it');
ok(tvRow.includes('row-summary'), 'TV row: the summary slot stays, empty');
TV.active = false;
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

// ── The search shortcut, TV mode included ───────────────────────────────
const key = (k, mod = {}, tag = 'BODY') => ({ key: k, target: { tagName: tag }, ...mod });
ok(H.wantsSearchFocus(key('f', { ctrlKey: true })) === true, 'Ctrl+F asks for the search box');
ok(H.wantsSearchFocus(key('F', { metaKey: true })) === true, 'Cmd+F too, and the shifted letter');
ok(H.wantsSearchFocus(key('/')) === true, '"/" on its own asks for it');
ok(H.wantsSearchFocus(key('f')) === false, 'a bare f is just a letter');
ok(H.wantsSearchFocus(key('/', {}, 'INPUT')) === false, '"/" inside a field is a character, not a shortcut');
ok(H.wantsSearchFocus(key('/', { ctrlKey: true })) === false, 'Ctrl+/ is something else');
// Typing in a field stays typing, in either mode.
TV.active = true;
ok(H.wantsSearchFocus(key('/', {}, 'INPUT')) === false, 'not while typing in the TV field');
TV.active = false;

/* Where the shortcut lands is the part that regressed — wantsSearchFocus never
   reads TV.active, so asserting it twice under both modes pinned nothing at all.
   Deleting the TV branch has to fail a test, and this is that test. */
TV.active = false;
assert.deepStrictEqual(H.searchTarget(), { id: 'search', openPanel: false },
  'out of TV mode the shortcut goes to the header box and opens no panel'); checks++;
TV.active = true;
assert.deepStrictEqual(H.searchTarget(), { id: 'tv-search', openPanel: true },
  'in TV mode it goes to the panel twin, and the panel has to be opened first — '
  + 'the header box is inside a display:none element and cannot take focus'); checks++;
TV.active = false;

/* Pressing the shortcut a second time hands the press to the browser. The
   handler can only do that by NOT calling preventDefault, so the two halves
   have to agree: wantsSearchFocus still has to claim Ctrl+F inside a field,
   or the press would fall out of the handler before anyone decided anything. */
const inBox = (k, mod = {}, id = 'search') =>
  ({ key: k, target: { tagName: 'INPUT', id }, ...mod });

ok(H.wantsSearchFocus(inBox('f', { ctrlKey: true })) === true,
  'Ctrl+F inside the box is still the shortcut — the way out is wantsBrowserFind, not here');
ok(H.wantsBrowserFind(inBox('f', { ctrlKey: true })) === true,
  'a second Ctrl+F, from the box the first one focused, is for the browser');
ok(H.wantsBrowserFind(inBox('F', { metaKey: true })) === true, 'Cmd+F the same way');
ok(H.wantsBrowserFind(key('f', { ctrlKey: true })) === false,
  'the first press comes from the page, and belongs to us');
ok(H.wantsBrowserFind(inBox('f')) === false, 'a bare f in the box is a letter being typed');
ok(H.wantsBrowserFind(inBox('/')) === false, 'and so is a second slash');

// Which box counts is whichever one the shortcut would have focused.
TV.active = true;
ok(H.wantsBrowserFind(inBox('f', { ctrlKey: true }, 'tv-search')) === true,
  'in TV mode the panel twin is that box');
ok(H.wantsBrowserFind(inBox('f', { ctrlKey: true }, 'search')) === false,
  'the header box is unreachable in TV mode, so a press from it is not a second press');
TV.active = false;

// ── Chip selection: a click toggles, with no modifier to hold ────────────
/* The same rule for severities, statuses and sources — the source row already
   worked this way, and the other two now share its one implementation. */
const sel = (...init) => new Set(init);
const picked = (s, v, clear) => { H.applyChipSelection(s, v, clear); return [...s].sort(); };

assert.deepStrictEqual(picked(sel(), 'critical'), ['critical'],
  'a click on an empty selection selects that one'); checks++;
assert.deepStrictEqual(picked(sel('critical'), 'error'), ['critical', 'error'],
  'a second click adds to the selection — no modifier needed'); checks++;
assert.deepStrictEqual(picked(sel('critical', 'error'), 'error'), ['critical'],
  'clicking a selected chip takes it out'); checks++;
assert.deepStrictEqual(picked(sel('critical'), 'critical'), [],
  'and taking the last one out is no filter at all'); checks++;
assert.deepStrictEqual(picked(sel('critical', 'error', 'warning'), 'all', true), [],
  'the all chip clears everything'); checks++;
assert.deepStrictEqual(picked(sel(), 'all', true), [],
  'clicking all when nothing is selected changes nothing'); checks++;
/* Clearing is asked for explicitly, not by a reserved value: a source really
   named `all` used to wipe the selection instead of joining it, and could never
   render as active because the set never contained it. */
assert.deepStrictEqual(picked(sel('Zabbix'), 'all', false), ['Zabbix', 'all'],
  'a source or severity named "all" is an ordinary value'); checks++;
assert.deepStrictEqual(picked(sel('all'), 'all', false), [],
  'and can be taken out again like any other'); checks++;

// Round-trips through localStorage and the URL, legacy single values included.
assert.deepStrictEqual([...H.parseFilterSet('critical,error')].sort(), ['critical', 'error']); checks++;
assert.deepStrictEqual([...H.parseFilterSet('critical')], ['critical'],
  'a value stored by an older version is a one-element set'); checks++;
assert.deepStrictEqual([...H.parseFilterSet('all')], [], '"all" is the empty set'); checks++;
assert.deepStrictEqual([...H.parseFilterSet(null)], [], 'and so is nothing at all'); checks++;
ok(H.formatFilterSet(new Set()) === 'all', 'the empty set is written back as "all"');
ok(H.formatFilterSet(new Set(['a', 'b'])) === 'a,b', 'and a selection as a list');

// ── Silenced and inhibited are filtered apart, and combine ──────────────
/* One show/hide toggle used to cover both, so "what is another alert masking
   right now?" could not be asked at all. `firing` stands for "not suppressed",
   so a pending alert rides with it rather than needing a chip of its own. */
const savedData = App.data;
App.data = { alerts: [
  { name: 'Firing',    status: 'firing',    severity: 'critical', source: 'A', labels: {}, annotations: {} },
  { name: 'Silenced',  status: 'silenced',  severity: 'warning',  source: 'A', labels: {}, annotations: {} },
  { name: 'Inhibited', status: 'inhibited', severity: 'warning',  source: 'A', labels: {}, annotations: {} },
  { name: 'Pending',   status: 'pending',   severity: 'info',     source: 'A', labels: {}, annotations: {} },
], sources: [{ name: 'A', status: 'ok' }] };

const shown = (...kinds) => {
  App.statusFilter = new Set(kinds);
  return H.filteredAlerts().map(a => a.name).sort();
};
assert.deepStrictEqual(shown('firing'), ['Firing', 'Pending'],
  'firing: neither silenced nor inhibited, but pending still counts'); checks++;
assert.deepStrictEqual(shown('silenced'), ['Silenced'],
  'silenced alone: firing excluded'); checks++;
assert.deepStrictEqual(shown('inhibited'), ['Inhibited'],
  'inhibited alone'); checks++;
assert.deepStrictEqual(shown('silenced', 'inhibited'), ['Inhibited', 'Silenced'],
  'both suppressed kinds together, which one chip each could not express'); checks++;
assert.deepStrictEqual(shown('firing', 'silenced'), ['Firing', 'Pending', 'Silenced'],
  'firing plus silenced, leaving the inhibited ones out'); checks++;
assert.deepStrictEqual(shown(), ['Firing', 'Inhibited', 'Pending', 'Silenced'],
  'the empty selection is no filter at all'); checks++;

// ── Several severities at once ──────────────────────────────────────────
App.statusFilter = new Set();
const bySev = (...sevs) => {
  App.sevFilter = new Set(sevs);
  return H.filteredAlerts().map(a => a.name).sort();
};
assert.deepStrictEqual(bySev('critical'), ['Firing'], 'one severity'); checks++;
assert.deepStrictEqual(bySev('critical', 'info'), ['Firing', 'Pending'],
  'two severities at once'); checks++;
assert.deepStrictEqual(bySev(), ['Firing', 'Inhibited', 'Pending', 'Silenced'],
  'no severity selected is every severity'); checks++;
// An alert whose source sent no severity is counted under `none`, so the `none`
// chip has to find it — it used to match nothing at all.
App.data.alerts.push({ name: 'Bare', status: 'firing', severity: '', source: 'A', labels: {}, annotations: {} });
assert.deepStrictEqual(bySev('none'), ['Bare'], 'the none chip finds an alert with no severity'); checks++;
App.data.alerts.pop();

/* A selected severity that nothing carries still gets a chip, at zero. The
   chips are the only visible trace of the filter, so without this a stale or
   mistyped severity left an empty dashboard with no cause on screen and no
   chip to click off. */
App.sevFilter = new Set(['sev-renamed-away']);
ok(H.severityCounts().some(([s, n]) => s === 'sev-renamed-away' && n === 0),
  'a selected severity absent from the data is listed at zero');
App.sevFilter = new Set();
ok(!H.severityCounts().some(([s]) => s === 'sev-renamed-away'),
  'and disappears once it is deselected');

/* An empty list under a suppressed-only selection is an answer — "nothing is
   silenced" — not the green tick of an unfiltered all-clear. */
App.statusFilter = new Set(['silenced']);
App.data.alerts = [];
ok(!H.emptyStateHtml().includes('✅'), 'only-silenced and empty: no all-clear tick');
ok(H.emptyStateHtml().includes('No silenced alerts'), 'and it says what is empty');
App.statusFilter = new Set(['silenced', 'inhibited']);
ok(H.emptyStateHtml().includes('No silenced or inhibited alerts'),
  'both kinds selected: both named, in chip order');

/* But a source that has not answered comes first, whatever the filters say.
   Folding the status selection into the `filtering` flag made "No silenced
   alerts" outrank "unreachable: Zabbix" — asserting nothing is silenced while
   the source that would know never replied. */
App.statusFilter = new Set(['silenced']);
App.data.sources = [{ name: 'AM', status: 'ok' }, { name: 'Zabbix', status: 'error' }];
ok(H.emptyStateHtml().includes('unreachable: Zabbix'),
  'a failing source is reported even under a status filter');
App.data.sources = [{ name: 'AM', status: 'ok' }, { name: 'Zabbix', status: 'pending' }];
ok(H.emptyStateHtml().includes('Waiting for Zabbix'),
  'and so is one that has not answered yet');
App.searchQ = 'nothing';
ok(H.emptyStateHtml().includes('Waiting for Zabbix'),
  'a search does not get to claim "no results" either while a source is missing');
App.searchQ = '';
App.statusFilter = new Set(['firing']);
App.data = savedData;

// ── Timezone ────────────────────────────────────────────────────────────
AppConfig.timezone = 'Europe/Nowhere';
assert.doesNotThrow(() => new Date().toLocaleTimeString('en-US', { ...H.tzOptions() }),
  'an unknown timezone must not throw — it used to kill the TV clock every second');
AppConfig.timezone = 'local';

console.log(`static/app.js: ${checks} assertions passed`);
