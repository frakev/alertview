# Changelog

All notable changes to AlertView are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.13.3] - 2026-09-29

### Changed
- **The documentation is eight pages instead of twenty-three.** `docs/` was 9,000 lines across six folders, much of it generic advice, duplicated between pages or describing things AlertView does not do. It is now: [Getting started](docs/getting-started.md), [Configuration](docs/configuration.md), [Display](docs/display.md), [Deployment](docs/deployment.md), [Troubleshooting](docs/troubleshooting.md), [API](docs/api.md) and [Development](docs/development.md), about 850 lines in all. The README went from 529 lines to a short presentation pointing to them.
- `alertview --help` lists `--config <FILE>`, which worked but was documented nowhere in the help.

### Fixed
- **Documentation errors removed with the rewrite**: `bearer_token: "${GRAFANA_TOKEN}"` recommended as a best practice by the environment variables page, which said three paragraphs earlier that it is sent literally; `tls_insecure: true` as a per-source option in an example, which AlertView refuses; the advice to add a private CA to the system's certificate store, which AlertView does not read (it trusts the bundled public authorities only); `500` responses from `/api/alerts`, which never happen, while the `429` of `/events` and its `config_error` event were missing; serving AlertView under a sub-path, which cannot work since the page loads its files from the root; the statement that ConfigMap edits need a pod restart; an HPA and a persistent volume for a stateless single pod; multi-architecture images, which are `linux/amd64` only; and tests, benchmarks and a Codecov upload that do not exist.
- **The documentation checks cover more**: internal links are checked down to the heading they point to, and every YAML block marked `alertview-config` must be a configuration AlertView accepts — the examples page claimed this was already the case.

## [0.13.2] - 2026-09-29

### Changed
- **A TV row shows one sentence, not two.** With `show_alert_name: false`, the annotation used as the title — the description, with `title_annotations: description` — was followed on the same line by the summary in grey. Once an annotation is the title, the row no longer adds the summary beside it; its column stays, empty, so rows still line up. Cards are unchanged: they have the room, and keep the summary below the title.

### Fixed
- **Documentation caught up with 0.13**: the polling section says each source is published as it answers and what `pending` means; the source `status` lists `pending` in the README; the unknown-key section shows the top-level display option hint; troubleshooting explains the ⏳ and ⚠ empty states and points configuration errors at the unknown-key message; `display-options.md` says `title_annotations` only applies with `show_alert_name: false`, and that new alerts are tracked per source. `SECURITY.md` listed 0.10.x as the supported version, described "an optional cache" that no longer exists, and did not say which outside host the CSP allows.
- `docs/development/testing.md` described a `src/cache.rs`, a `tests/*.rs` integration crate, benchmarks and a Codecov upload, none of which exist, and did not mention the frontend tests. It now lists the two real suites and the exact commands CI runs. The project tree in `docs/development/README.md` lost its `cache.rs` too.

## [0.13.1] - 2026-09-29

### Security
- **`rustls` 0.23.45** (from 0.23.40), for [RUSTSEC-2026-0285](https://rustsec.org/advisories/RUSTSEC-2026-0285): TLS 1.3 handshake messages were accepted across encryption level boundaries. AlertView uses it for every connection to a source. The advisory is dated 2026-09-14 and was caught by the audit added to CI in 0.13.0, on its first run.

### Changed
- **A display option written at the top level says where it goes.** `play_sounds: true` next to `sources:` — ignored in silence before 0.13.0, refused since — now reads ``play_sounds — a display option: move it under `display:` (`display.play_sounds`)``. The check asks `DisplayConfig` itself whether it knows the key, so it cannot drift from the options that exist.

### Fixed
- **The dependency audit could not report in CI.** `rustsec/audit-check` publishes a check run, which needs `checks: write`; the workflow grants `contents: read` only, and a pull request from a fork never gets more. It failed with "Resource not accessible by integration" whatever it found. CI now runs `cargo audit` itself, and the step fails on an advisory with no permission at all.

## [0.13.0] - 2026-09-29

### Security
- **An unauthenticated request no longer reaches your monitoring systems.** `/api/alerts` used to fetch upstream on every request: with caching off — the default — one anonymous HTTP call to AlertView became four to seven calls to Alertmanager, Grafana and Zabbix, and the load grew with the number of people watching. AlertView now polls on a schedule of its own and serves every browser from the result. **Measured: 50 concurrent dashboard requests → 0 upstream requests**, against 50 before, and the handler answers in 0.3 ms.
- **A Content-Security-Policy, with `script-src 'self'` and no `unsafe-inline`** — which is what actually stops an injected `onmouseover=` from running, the shape of the escaping bug fixed in 0.10.0. The two scripts that were inline in `index.html` moved into `/theme.js` and `app.js` to make that possible. `style-src` still allows inline styles: six generated `style=` attributes remain, and inline CSS cannot execute. The custom stylesheet's host — `display.custom_css`, or a `theme` holding a URL — is added by origin to `style-src`, and to `font-src` and `img-src` for the fonts and images it loads. When a hot reload points it at another host, the page reloads itself to pick up the new policy. Also `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer` — the dashboard URL, filters included, no longer travels to the runbooks it links to — and `X-Frame-Options: DENY`.
- **Upstream responses are capped at 64 MiB.** The per-source `timeout` bounded how long a fetch could take, not how many bytes it could deliver; `Content-Length` is checked before a single byte is read, and the body is refused as soon as it passes the ceiling.

### Added
- **`display.title_annotations`** — which annotation takes the place of the alert name when `show_alert_name` is false, instead of always `summary`. A list tried in order, the first one the alert carries wins: `["description", "summary"]` shows the description, and the summary for alerts that have none. The annotation used as the title is not repeated below it. A single string is accepted too (`title_annotations: description`). Defaults to `["summary"]`, so existing configs are unchanged.
- **Tests for the server's core.** `get_alerts` and the poller had none. The important one asserts that **20 dashboard requests produce exactly one upstream request** — the property the background poller exists for, now guarded rather than measured by hand. Also a failing source being reported rather than hidden, and new alerts being announced once.
- **The frontend is tested in CI.** `tests/frontend/render.test.js` runs the real functions out of `static/app.js` — 56 assertions covering the escaping (a hostile label must produce no executable attribute), the nine-slot TV grid, the icons, the severity ordering, the label filters and the timezone guard. CI ran `node --check` before, which would not have caught any of the bugs this release fixes.
- **Three scanners that tie the documentation to the code**, because the drift happened where nothing connected them: every `{{.Placeholder}}` in the docs must be one `apply_link_template` substitutes, every `ALERTVIEW_*` must be one the code reads **and** every variable the code reads must be documented, and every internal link must resolve. They caught a fake `ALERTVIEW_TLS_INSECURE` and two real variables documented nowhere on their first run.
- `cargo fmt --check` and a RustSec advisory scan in CI. 137 files were unformatted; the two advisories of last week were found by hand.
- **A key AlertView does not understand now stops it from starting**, with its full path and, where the option genuinely moved, what to write instead. It used to be dropped in silence — a config carrying six of the keys the old docs taught started cleanly and ignored all of them. Captured with `serde_ignored` rather than `deny_unknown_fields` on each struct, so there is no attribute to forget on the next struct added, and so a path comes out as `sources[0].cache_ttl` rather than a bare `cache_ttl` that says nothing about which source it came from.
- **A refused reload is visible in the dashboard**, as an amber banner naming the reason, and is broadcast over SSE as `config_error`. The running configuration is kept, as before; until now the only trace was a log line, which on a wall display means nobody sees it.

### Changed
- **`cache_ttl_seconds` is now the polling interval** rather than a per-request cache lifetime. It keeps its meaning — "do not hit the sources more often than this" — and `0` now means "poll at `refresh_interval`" instead of "never cache". A configuration reload re-polls at once rather than at the end of the current interval.
- **A slow source no longer holds a browser.** With caching on, requests used to queue behind a single-flight gate that covered the whole retry sequence — up to 67 seconds with the default timeout and backoff. The dashboard now answers from the last successful poll and marks the source as failed. Measured: 0.5 ms while a source was timing out. Each source is published as soon as it answers, so a slow one does not hold back the others; at startup it is listed as `pending`, and the empty list says what it is waiting for instead of showing the ✅ "no active alerts". New-alert sounds and notifications are tracked per source, so a source coming back — after startup or an outage — does not announce its whole backlog.
- **`/`, `/style.css`, `/app.js` and `/theme.js` carry `Cache-Control: no-cache`.** They carried no directive at all, so browsers cached them heuristically and an upgrade needed a forced refresh to take — which matters more now that the stylesheet carries the icons. `no-cache` still stores the copy; it just confirms it, so the cost is a 304 rather than a re-download.
- **The search waits for the typing to settle** (120 ms) instead of rebuilding the whole list on every keystroke.
- **The Kubernetes deployment pins its image version** instead of tracking `:latest` with `imagePullPolicy: IfNotPresent`, a pairing under which a restart silently keeps whatever the node already had — a rollout that never rolled anything out. It also carries the full restricted `securityContext` the documentation has been describing: non-root 65532, read-only root filesystem, all capabilities dropped, `RuntimeDefault` seccomp.
- Links keep `rel="noopener noreferrer"` in kiosk mode, where they open in the same tab. Without it the dashboard URL, filters included, travelled in the `Referer` to whatever the alert pointed at.
- The frontend validates a link's scheme itself rather than trusting the server to have done it: the same `http(s)`-only check `sanitize_link` applies, on the way out.
- The shipped ConfigMap says in its first lines that it is an example, and that real tokens belong in a Secret.

### Fixed
- **Alerts from different sources interleaved wrongly.** Ordering compares timestamps as strings, and the sources disagree on the spelling: Alertmanager sends `…Z`, Zabbix produced `…+00:00`, and fractional seconds come and go. `10:00:00.5Z` sorted *before* `10:00:00Z` because `.` is below `Z`, and an alert carrying a `+02:00` offset sorted hours from where it belonged. Every timestamp is now normalised to UTC RFC 3339 as it is read.
- **A CSS class the stylesheet never knew.** TV rows emitted `tv-lbl` on every label chip and there was no rule for it — removed rather than invented, so nothing changes visually.
- **Dead and contradictory CSS**: `.src-flt-chip.active` was declared twice outside any media query, four of the first block's five properties never applying; `.alert-row` set `flex-wrap: nowrap` and then `wrap` forty-eight lines later; `.alert-row .src-chip` could never match, since `src-chip` is only emitted on cards and `.alert-row` only on TV rows; `.header` and `.tv-overlay` repeated values their own blocks had already set.
- The ↗ tooltip on a card omitted the source name, because `genLinkHtml` was called with two of its three arguments — the TV row passed all three.
- Dead JavaScript: `sseConnected` was written twice and never read, `window._eventSource` was stored "for cleanup" that does not exist, and `TV.chosen` was never declared on the object it lives on.
- **Source URLs were logged with their credentials** on every reload, in the fallback polling watcher (`main.rs`). Three of the four log sites had been redacted and the fourth had drifted — the earlier fix was verified by counting the redacted sites rather than the total. The reload path is now written once instead of three times, which removes the class of drift rather than the instance.
- The README contradicted itself in a single section: it claimed a `ghcr.io/…:main` tag and a `self-hosted` `k8s-home` runner three lines after correctly stating that publication is tag-only and while every job runs on `ubuntu-latest`. It also said all endpoints except `/health` are gzip-compressed, when the layer is applied to the whole router and it is `/events` that is exempt.
- Six dead internal links, three of them in the documentation index, plus a `*Last updated: $(date +%Y-%m-%d)*` that was never substituted.
- `${GRAFANA_TOKEN}`-style interpolation was still shown in `config-file.md` and `source-types.md`, which contradicted `kubernetes.md` correctly stating that AlertView does not expand variables in the YAML.
- `ALERTVIEW_CONFIG_PATH`, which does not exist, in the Kubernetes staging example and in the testing guide. The variable is `ALERTVIEW_CONFIG`.
- Three endpoints that never existed (`/api/sources`, `/api/config`, `/api/alerts/alertmanager`) in the troubleshooting guide, and one more `refresh_interval` shown under `display:`.

### Removed
- Twelve `#[serde(default)]` attributes on `AlertsResponse`, which derives `Serialize` only — defaults apply when reading, and nothing reads it. They read as meaningful and were not.
- Three copies of the "empty the search box" body, which had already started to differ, and two copies of the severity-chip builder that differed only by an inline style. The two row toggles were the same function twice over a different set.
- **The documentation that described options AlertView never had.** 71 % of `docs/` came from a single commit that was never reconciled with the code, and it recommended `display.filters`, `display.sort`, `display.compact_mode`, `display.hide_header`, `display.hide_footer`, `display.severity_colors`, a per-source `cache_ttl` and a per-source `tls_insecure` — none of which exist — along with `alertview --port`, `/api/sources`, `${VAR}` interpolation inside the YAML, and Grafana/Zabbix link placeholders (`{{.TriggerID}}`, `{{.DashboardUID}}`…) that make the link disappear instead of failing. 4 895 lines deleted: `docs/faq.md`, `docs/configuration/advanced.md`, `docs/development/structure.md` and six of the seven files under `docs/examples/`. `docs/examples/README.md` was rewritten as three configurations that are actually parsed. The reference has always been `config.example` and `docs/configuration/config-file.md`; they stay.

## [0.12.0] - 2026-09-10

### Changed
- **`flame`, `bell-off` and `hourglass` are now the emoji artwork itself**, embedded rather than drawn by hand. The hand-drawn silhouettes of 0.11.0 fixed the rendering but did not look like the emoji they replaced, which was the point of having them. AlertView now ships the Noto Color Emoji drawings for 🔥, 🔕 and ⏳ as `data:` URIs in the stylesheet: the dashboard looks like it did before 0.11.0, and it looks the same on every machine, including the ones with no emoji font. A typed emoji is still accepted and still means "use whatever font this machine has".
  - Embedded as data URIs rather than inline SVG on purpose: each icon is then its own document, so the flame's two gradient ids cannot collide with the copy of themselves on the next row, and the artwork travels once with the stylesheet instead of being repeated in the markup of every alert. The three add ~9 KB to `style.css`, which is served compressed.
  - **Licensing**: the drawings are © Google LLC under the SIL Open Font License 1.1, recorded in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) with the upstream commit they were taken from, the full licence text in `LICENSES/`, and the modifications made. Upstream is not self-consistent — its README claims Apache-2.0 for image resources while linking to a file containing the OFL — so AlertView complies with the licence file that is actually there, which satisfies both readings. The SVGs were minified for embedding; the drawings are untouched, verified by comparing the parsed element trees before and after.

## [0.11.1] - 2026-09-10

### Fixed
- **The flame was the wrong colour, and changed colour once clicked.** The severity marker sits inside the runbook link when one is configured, and `.mark-link` declared no `color` of its own — so the icon inherited the browser's link colour, turning dark purple after the first visit. The emoji it replaced was a coloured glyph and immune to it; a `currentColor` icon is not. Links no longer repaint what they wrap, and the flame is painted deliberately: the body in the critical red, over a hot amber core. A single flat colour read as a leaf rather than as fire, which is what made it look nothing like the emoji.

## [0.11.0] - 2026-09-10

### Changed
- **A critical row is now tinted about twice as strongly** — `.07` to `.12`, and `.14` to `.20` in TV mode. At `.07` it was indistinguishable from an `error` row sitting at the same value, which defeats the purpose of colouring the row at all: it is meant to be read from across the room, not compared side by side.
- **Critical stands out more, and is finally legible on the light theme.** The dark theme goes from `#f85149` to `#ff4438` — fully saturated, a shade deeper, the same contrast against the background. The light theme had no critical colour of its own, so it inherited a red that sat at **3.35:1 on white**, under WCAG AA for text — and a critical badge *is* text; it now uses `#d1242f`, at 5.24:1. TV mode is untouched, it already had its own brighter set.
- **The severity colours are declared once.** Their translucent backgrounds were `rgba()` literals repeating the same channel numbers in 28 places, so changing a severity left half the interface on the old hue — a badge in the new red on a tint of the old one. Each severity now carries a `--c-*-rgb` companion token. Verified by resolving every one of the 56 `rgba` lines back to its former value: only the six critical ones move.

### Fixed
- **The critical flame did not render everywhere.** `critical_icon` defaulted to `🔥`, which only appears if the machine showing the dashboard has a colour emoji font — a kiosk browser, a minimal Linux box or a Windows N edition draws an empty box, on exactly the screen that most needs to be readable. The defaults are now drawn rather than typed: `flame`, `bell-off` and `hourglass` are built-in inline SVG that always render, take the severity colour from the surrounding text and stay sharp when a wall display scales them up. `status_icons` had the same exposure with `🔕` and `⏳`, and gets the same treatment. Any other value is still rendered as text, so `critical_icon: "🔥"` keeps the emoji for anyone who wants it.

## [0.10.3] - 2026-09-09

### Fixed
- **The age of an alert was illegible on the light theme** — `.time-ago` was painted with `--c-warning` (`#d29922`), an amber that reads as a warm accent on the dark background but turns to mud on white. It now goes through a `--c-time` token: the same amber on the dark theme, the ordinary text colour (`#1f2328`) on the light one.

## [0.10.2] - 2026-09-04

### Fixed
- **The browser tab counted alerts the filters had hidden** — narrowing 54 alerts to 3 with a search still read `54 alerts` in the tab, because the title counted every firing alert whatever the filters said. It now shows what the toolbar shows, from the same two numbers: `🟡 3 / 54 alerts`, and the plain `54 alerts` when nothing is filtered out. A filter that matches nothing reads `🔍 0 / 54 alerts` rather than borrowing the colour of a severity it is not showing.
- **The tab was only refreshed after a poll** — `updateTitle()` was called from `render()`, which runs on a fetch, so typing in the search box, clicking a severity chip or toggling silenced alerts left the old number in place for up to 30 seconds. It is called from `renderAlerts()` now, the one function every filter change goes through.
- **The Docker workflow no longer sets up QEMU.** It builds `linux/amd64` and nothing else, so the emulation layer contributed nothing — but the step pulls `tonistiigi/binfmt` from Docker Hub on every run, which is a rate limit waiting to happen. It duly failed on the `v0.10.1` tag and skipped the build behind it.

## [0.10.1] - 2026-09-04

### Added
- **`--version` / `-V`** — the version was readable in the dashboard footer, in the TV status bar and in the `User-Agent`, but not from the binary itself. It is answered before the configuration is read and before the port is bound, so it works on a machine with neither. `--help` now names the version too, and the startup log line reads `Starting AlertView 0.10.0 on port 8080 with 3 source(s)` — "which build is this?" is the first question when a dashboard misbehaves.

### Changed
- **Security policy rewritten for a public repository** — it pointed reports at `security@frakev.com`, a domain with neither an MX nor an A record, so every report would have bounced. Reports now go through GitHub's private vulnerability reporting, with a public issue as the fallback. The policy also states what AlertView assumes about its environment: it has no authentication of its own and is meant to sit behind a proxy, so an unauthenticated `/api/alerts` is not a vulnerability — while anything an *alert* can do to a viewer is. The supported-versions table still said 0.4.x.
- **Mount the configuration read-only** — the documentation insisted on `:rw` "to enable auto-reload", in eleven places across the README and six guides. It is not true: AlertView only ever reads the file, and a `:ro` bind mount picks up host changes perfectly well (verified against the published image). It was telling everyone to mount a file holding their source credentials read-write for no reason. The real caveat — an editor that saves by renaming replaces the inode, which a single-file bind mount does not follow — is documented instead, with the directory mount that avoids it.

### Security
- **Dependencies updated for two advisories** flagged once the dependency graph was reviewed: `anyhow` 1.0.102 → 1.0.104 (RUSTSEC-2026-0190, unsoundness in `Error::downcast_mut` — AlertView only ever calls `downcast_ref`, so it was not reachable) and `quinn-proto` 0.11.14 → 0.11.17 (RUSTSEC-2026-0185 / CVE-2026-25800, remote memory exhaustion over QUIC — the crate is recorded in `Cargo.lock` but is not built for any target here and appears nowhere in the binary). Neither was exploitable in AlertView; both are gone from the lock file. The full lock (204 crates) now matches no advisory in the RustSec database.

### Fixed
- **`alertview --help | head` panicked** with `failed printing to stdout: Broken pipe`. The help text is built as one string and written in a single call that ignores a closed pipe, rather than printed line by line. `--help` also listed four of the seven environment variables the configuration reads; a test now checks that every flag and variable appears.
- **CI/CD documentation** — the README claimed the image was published to GHCR on every push to `main`, in two places; it has been tag-only since `d03c7a4`. The other two workflows were not described at all.
- Dropped a redundant "Verify GHCR authentication" step that re-ran `docker login` by piping a secret through a shell, right after `docker/login-action` had already logged in — pointless work, and in a public repository the job log is world-readable.
- `ci.yml` declares `permissions: contents: read` instead of inheriting the repository default.

## [0.10.0] - 2026-09-04

### Security
- **Alert content could inject HTML attributes** — the escaping helper round-tripped through `textContent`/`innerHTML`, which is the DOM's *text node* serialisation: it escapes `&`, `<` and `>` and leaves quotes alone. Every value was also interpolated into attributes (`title=`, `data-sev=`, `data-group-key=`, `href=`), so an alert label, an alert name, a severity or a silence author holding a double quote closed the attribute and turned the next word into a live event handler — stored XSS reachable by anyone able to set a label on a metric or write a silence comment. Escaping now covers `"` and `'` as well, and is regex-based rather than allocating a DOM element per call.

### Added
- **A dead backend no longer looks like a quiet one** — a failed poll used to be a line in the console: the countdown froze mid-number, "last refresh" kept showing an old time, and the alerts on screen were simply the last ones that arrived. A red *Backend unreachable — data frozen since HH:MM* banner now says so, the list is dimmed, the browser tab reads `⚠ stale`, and the countdown restarts on the 15-second retry instead of stopping. Everything clears by itself on the next successful poll. A *source* that fails is still reported separately, next to its name.
- **Keyboard access** — the severity chips, the source chips and the group headers are spans, not buttons: they were unreachable without a mouse. They now take focus, respond to Enter and Space, carry `aria-pressed` / `aria-expanded`, and have a visible focus ring, as do the links and toggles inside a row. The search box gained a label.

### Changed
- **Zabbix 6.0 through 7.x, without configuration** — `trigger.get` was called with `selectGroups`, deprecated in 6.4 and **removed in 7.0**, where the call fails and takes the whole source down with it. AlertView now tries `selectHostGroups` first and falls back to `selectGroups`, and does the same for `problem.get`'s `selectAcknowledgements` / `selectAcknowledges`; what worked is remembered per source, so an older server pays one extra round-trip once rather than on every poll. Only an "invalid params" answer triggers a fallback — an authentication failure surfaces immediately instead of being retried three times under different names.
- **Cache misses no longer stampede** — with `cache_ttl_seconds` set, every browser that arrived while an entry was expired fired its own upstream request. One fetch per source is now in flight at a time and the others wait for it: measured 6 concurrent dashboard polls → 1 upstream request, against 6 before.
- **Graceful shutdown** — `SIGTERM` and `Ctrl-C` stop new connections, let in-flight requests finish and close the SSE streams before exiting (measured: 5 ms with an open event stream). A Kubernetes rolling update no longer cuts requests mid-flight.
- **Invalid configuration is refused, not ignored** — `log_format`, `config_watch_method` and `config_poll_interval` were free-form: a typo silently selected the other branch. They are validated on startup and on reload, as is a non-empty `display.severity_order` (an empty one ranked every severity the same and quietly disabled sorting). A reload that fails validation keeps the running configuration.

### Fixed
- **Zabbix acknowledgement comments never reached the UI** — the acknowledgement parser required a `useralias` field that no Zabbix version returns on an acknowledgement (the object carries a `userid`; the name needs a separate call). serde failed on it, which failed the whole `problem.get` response, which fell through to a query without acknowledgements at all — so every ACK'd alert showed the generic "Acknowledged in Zabbix". Acknowledgement fields are now all optional, and the author's name is resolved with one `user.get` per poll; a token that may not read users simply leaves the author out.
- **One malformed alert took down a whole source** — the Alertmanager response was deserialised as a single array, so an entry missing a field turned "N alerts" into "error" with nothing shown. Alerts are parsed one at a time now: the unreadable one is logged and skipped, the rest are displayed. Only `fingerprint` is still required.
- **Source errors leaked credentials** — `sources[].error` is served to every browser, and reqwest puts the failing URL in its message, userinfo included. A `http://user:pass@host` source now reports `http://***@host`, in the logs as well as in the API.
- **An invalid `timezone` froze the TV clock** — an IANA name with a typo makes `Intl` throw, and the clock's tick had no guard (unlike `absTime`): it died on its first run and threw once a second afterwards. The zone is validated once and falls back to the browser's, with a console warning.
- **Custom severities went silent** — the new-alert sound stopped at the most severe level present and played nothing when that level had no preset, which is the case for every renamed severity. The nearest preset in the configured order is used instead, more severe first. The notification and tab-title icons were keyed off the level *names* (`critical`/`error`/`high`) and always came out 🟡 for a custom scale; they now follow the configured rank — unchanged for the default order.
- **Two browsers polling at the same instant announced the same alerts twice** over SSE: the "already announced" set was read, diffed and written back in three steps. It is now diffed and recorded in one critical section.
- **Cache entries were never evicted** — a source renamed, removed or repointed left its entry (with its copy of the alerts) behind on every config reload.

### Documentation
- **Kubernetes guide corrected** — its "Secrets Management" section advised injecting source tokens with `secretKeyRef`, which has never worked: AlertView does not expand environment variables inside `config.yaml`, so the placeholder was read as the literal token. Replaced by the pattern that does work — the whole configuration file in a Secret, mounted where the ConfigMap went — with the `fsGroup` a restricted file mode requires. The "Monitoring" section told you to point Prometheus at `/metrics`, an endpoint that does not exist and never has; it now describes probing `/health` from the outside. The `securityContext` example ran as uid 1000 while the image ships 65532, and omitted `readOnlyRootFilesystem`, dropped capabilities and the seccomp profile — the published image runs under the full restricted context, which the corrected example now shows. Added the caveat that a value in `config.yaml` wins over the matching environment variable, which the shipped ConfigMap makes easy to trip over.
- **Credits** — the README now says who maintains AlertView and who writes the code.
- Zabbix version compatibility table and the acknowledgement-author lookup (`source-types.md`); the stale-data banner, in **Features** and as its own troubleshooting entry; the full list of configuration validation rules (`config-file.md`); credential redaction in the API reference; rolling updates and `SIGTERM` handling (`kubernetes.md`); keyboard navigation added to the shortcuts table. Fixed drifted commands in the troubleshooting guide: a `--dry-run` flag that does not exist, `refresh_interval` shown under `display:`, `cache_ttl` instead of `cache_ttl_seconds`, and an `/api/sources` endpoint that has never existed.

## [0.9.1] - 2026-09-04

### Added
- **Silence comments name their author** — the Alertmanager silence's `createdBy` was parsed and then thrown away; it now travels with the comment as a `silence_created_by` annotation and leads the revealed line (`alice · maintenance until 6pm`), as does the acknowledging user for Zabbix. The 💬 tooltip names them too, so you know who to ask without opening anything.

### Changed
- **Mobile pass** — the runbook link on the severity dot was a 13×13px tap target announced only by a hover effect, which a touchscreen has no way to show: the hit area grows to ~35px on coarse pointers, without moving anything, and the dot carries a permanent ring where hover is unavailable. TV mode falls back to the flowing layout below 700px, where nine aligned columns cannot fit and were being clipped rather than wrapped. The TV list now ends above the HUD instead of behind it, and the HUD wraps rather than running off the edge of a narrow screen. The severity filter chips come back on phones, on their own scrollable row, rather than being dropped entirely.

### Fixed
- **Zabbix alerts were left without a runbook link** — a `display.alert_link_template` written the usual way, around `{{.Labels.alertname}}`, resolved for Alertmanager and Grafana but never for Zabbix, whose labels carry no `alertname`. The problem name is now exposed under that key, so one template covers the three source types. It is not rendered as a chip, so nothing appears twice.

### Documentation
- **Docs brought in line with 0.9.0** — seven places still described the whole card as the link rather than the severity dot (README, `features.md`, `config-file.md` in four spots, `source-types.md`, `api.md`); `status_icons` was missing from the configuration reference and from the API response schema; the README claimed a status filter that does not exist, and the TV mode description predated the permanent status bar. A **Keyboard Shortcuts** section was added — there was none, although `T` and `Escape` have been bound for a long time — now listing `Ctrl+F` / `Cmd+F`, `/`, `Escape` and `T`.

## [0.9.0] - 2026-09-04

### Added
- **The runbook link moved onto the severity dot** — the whole card used to be a link, which meant clicking anywhere navigated away and nothing said so. The dot (or the critical icon) is now the link, and it grows and lights up on hover to announce itself. The ↗ button keeps its own destination, unchanged.
- **`display.status_icons`** — the `firing` / `silenced` badges are gone from the alert rows: `firing` is the norm and saying so on every line is noise. Only the exceptions are marked, with an icon per status (`silenced: "🔕"`, `pending: "⏳"` by default, `""` to disable).
- **Silence comments behind a button** — the comment was printed inline in the row; a 💬 button next to the status icon now reveals it on its own full-width line, and the open state survives the refresh.
- **Ctrl+F / Cmd+F focuses the search box**, as does `/`, and Escape clears it then leaves it. The browser's find-in-page only ever finds what is already rendered, which is not what you want on a filtered list. Not bound in TV mode, where the search box is hidden and the native search stays available.
- **The TV HUD keeps a permanent minimal half** — source dots, clock, last refresh time and version are always on, since that is the "everything is fine" glance of a wall display; the theme, filter and exit buttons hide behind a `+`. The bar no longer fades out after four seconds of stillness.
- **Several values per label filter** — `team=sre|dba` matches either, and repeating a key does the same: `team=sre, team=dba`. Filters on different keys still combine with AND, so `team=sre|dba, severity=critical` narrows as expected. Negations are always AND-ed, so `team!=sre|dba` excludes both. Works with `~` too: `hostname~srv|db`.

### Changed
- **The version is always readable in TV mode** — it was dimmed to 35% and only came up on hover, which meant walking to the screen and finding a mouse to know what was running.
- **Prefix labels can no longer be truncated** — their column is sized to the longest prefix and never shrinks. When a row genuinely does not fit, it is the alert message that gives way, as it should.
- **The critical icon replaces the severity dot** instead of sitting next to it, so a critical reads at a glance; every other severity keeps its coloured dot, and so does a critical when `display.critical_icon` is empty. The marker is centred in its column, so dots and icons line up down the list.
- **Prefix labels are no longer capped** — the leftmost column was limited to 32 characters and truncated longer host paths. It now takes the width of the longest prefix in the list, and only gives way when the row genuinely runs out of room.

### Fixed
- **Revealed labels were cut off in TV mode** — they were rendered inside the labels column, which is capped so one alert cannot starve the rest of the row, so opening them clipped the chips mid-word. They now get a full-width second line under the row: nothing is truncated, and the columns of the row above stay in place. In the fallback layout (browsers without `subgrid`) they wrap onto their own line as well instead of overflowing the end of the row.

## [0.8.1] - 2026-09-04

### Changed
- **Severity and status badges come before the alert text** — they used to follow it, so their position depended on the length of the name; they now sit in a fixed column right after the prefix labels, in TV rows and on cards alike, and severity reads straight down a column.

## [0.8.0] - 2026-09-04

### Changed
- **TV rows line up in columns** — each row was its own flex line, so every column started wherever the previous element happened to end: with hostnames of different lengths the message, the badges and the labels were ragged from one row to the next. The alert list is now a CSS grid and each row a `subgrid` of it, so a column is as wide as the widest cell across all rows and the tracks size to their content, capped with `fit-content()` so one very long hostname cannot starve the message. No hardcoded widths — unlike the 0.5.3 attempt (`7rem 4.5rem 10rem`) that was reverted in 0.5.4. The age is right-aligned so `for 4m` and `for 28m` line up on their digits. Every row emits the same nine slots even when empty, which is what keeps the columns in step; a browser without `subgrid` support falls back to the previous flex layout untouched.

### Documentation
- **`config.example` rewritten as a complete, organised reference** — options had been appended as they were added, leaving the file unsorted and partly stale: a "Global timeout for all sources" comment sitting above `tls_insecure` (there is no such setting), a Zabbix `link_template` still using the `filter_eventid` parameter dropped back in 0.4, and `show_labels` / `show_alert_name` descriptions predating the reveal button. It is now grouped into Server / Sources / Display sections, every per-source option is documented on the first source, and the file ends with the full list of environment variables and of the URL parameters. All 42 configuration fields are covered.
- The two `filter_eventid` mentions in the README were replaced by how Zabbix links are actually built, and `docs/configuration/config-file.md` gained the per-source options missing from its inline schema.

## [0.7.1] - 2026-09-04

### Added
- **A button to reveal what the config hides** — `display.show_labels: false` and `display.show_alert_name: false` no longer make things simply disappear: each alert carries a small `+N` button bringing back its labels and, when the summary took its place, its name as an `alertname=…` chip. One button covers both, on cards as well as TV rows, and it reuses the toggle TV rows already had for the labels they have no space for. The open state is now kept per alert rather than in the DOM, so it survives the auto-refresh.

### Fixed
- **TV rows were laid out with two gaps** — with `display.show_alert_name: false` the summary took the place of the alert name and both it and the (now empty) `.row-summary` grew to fill the row, so the free space was split in two and the severity badges ended up stranded in the middle. The summary now shrinks but never grows, leaving a single spacer, and the badges sit right after the text as they do in every other layout.
- **Prefix labels were truncated too early** — the prefix was capped at 22 characters, which cut a row like `top1-mon-1 / coreiaas / top1-…` short even on a wide screen. TV rows show it in full and let it shrink (before the alert text does) only when the row actually runs out of room; cards keep a cap, raised to 40 characters. Its font size also matches the alert text now.

## [0.7.0] - 2026-09-04

### Changed
- **Sources are fetched concurrently** — `/api/alerts` used to take the sum of every source's latency; it now takes the slowest one (8 in flight at a time, results kept in config order). Measured with three sources answering in 2s: 2.0s instead of ~6s.
- **The config lock is no longer held across the network I/O** — the handler snapshots the config and releases the read guard before fetching, so a config reload no longer waits for the slowest source.
- **Groups no longer duplicate the alert payload** — `AlertGroup.alerts` repeated every alert already present in `alerts`, doubling the response when `group_by` was set. The frontend never read it: it picks the members out of the main list.
- **Groups are ordered by severity** — most severe group first instead of alphabetically by key, so on a wall display the team with a critical is at the top.
- **Sorting does less work** — the severity rank is computed once per alert instead of on every comparison, on both sides.
- **`display.source_link` and the SSE payload agree** — new alerts are broadcast after the config-wide link settings are applied, so an SSE payload carries the same alert as `/api/alerts`.
- **URL parameters no longer stick** — `?tv=1`, `?sev=`, `?src=` and `?silenced=` used to be written to `localStorage`, so opening a shared link once pinned that setting in the visitor's browser for good. They now apply to that visit only.
- **CI** — a workflow now runs `cargo clippy -- -D warnings`, `cargo test` and a syntax check of the static assets on every push and pull request; nothing but tag builds ran before. Two integration tests were added against a stub Alertmanager, covering the failures that actually happened: the API path being dropped from the source URL, `silencedBy` not deserialising, a `javascript:` generator URL reaching the frontend, and a 404 surfacing as a typed status error. `release.yml` no longer uses the archived `actions-rs/toolchain`.

### Fixed
- **The theme picked by the user was overwritten 30s later** — `display.theme` was re-applied on the next poll because the "the user chose this" flag was computed once at startup and never updated when the theme button was clicked.
- **Grouping hid alerts** — an alert missing the grouping label was placed in a `<missing>` group the frontend could never match, and a label value containing `,` or `=` scrambled the group key when it was re-parsed. Membership now comes from the labels the server sends. Measured on 6 alerts grouped by `team`: 2 were invisible and 2 were shown twice; all 6 now land in the right group.
- **A blocked storage made the page blank** — `localStorage` raises rather than returning null in private browsing or with site data blocked, and it was read unguarded at startup, taking the whole script down. All access goes through guarded helpers.
- **Clearing the search left `?q=` in the URL** — with label filters in the box, a reload silently brought the filter back.
- **Exponential backoff was never capped** — `max_delay_ms` was applied to the multiplier instead of the delay.
- **408 and 429 are retried again** — the "don't retry 4xx" rule covered them, although both mean "try again later" rather than "you are misconfigured".
- **`ALERTVIEW_CONFIG` is read** — it was documented in `--help` but never looked at. `--config <path>` is accepted as well.
- **Duplicate source names are rejected at startup** — they silently shared a cache key, an announced-fingerprint entry and a filter chip.
- **Inhibited alerts are labelled as such** — an alert suppressed by another alert (`inhibitedBy`) showed up as silenced with no comment, indistinguishable from a real silence.
- **HTML injection through the `severity` label** — the severity was interpolated raw into a CSS class, a chip's text and an inline `onclick`, so an alert carrying a crafted `severity` label (`x" onmouseover="alert(1)`) broke out of the attribute and injected a handler. Severity now goes through `esc()` for display and through a slug for the CSS class token. Same defect for source names and group keys, which broke the inline handler on any value containing a quote: the chips and group headers no longer carry inline `onclick` attributes at all, the values travel in `data-*` and one delegated listener per container handles the click.
- **Silence comments were never displayed** — `AmStatus.silenced_by` was missing its `#[serde(rename = "silencedBy")]`, so the field Alertmanager actually sends never deserialized and the silence lookup always came up empty. Silenced alerts now show the silence comment again.
- **A long summary broke the TV row layout** — with `show_alert_name: false` the summary landed in `.alert-name`, which has no flex or truncation, and pushed the trailing metadata out of the row. It now takes the free space and truncates with an ellipsis.

### Added
- **Label filters in the search box** — the search now accepts comma-separated `key=value` filters alongside free text: `team=sre, hostname~web`, `severity=critical, team!=dba`, or mixed with a plain search (`team=dba, slow queries`). `=` is an exact case-insensitive match, `!=` excludes (and matches alerts without the label), `~` means "contains". Keys are looked up in the alert's labels then its annotations, with `source`, `status`, `name`/`alertname` and `severity` also usable. Anything that is not a `key<op>value` pair stays free text, so the previous behaviour is unchanged. Filters live in `?q=` like before, so a per-team view can be bookmarked or put on a wall display.
- **`display.show_alert_name`** — set to false, the `summary` annotation takes the place of the `alertname` in the card title and the TV row, and is not repeated below. An alert without a summary keeps its name so a row is never blank.
- **`display.show_labels`** — hides the label chips (and the TV `+N` toggle) without having to empty `display.labels`. Prefix labels are unaffected.
- **`display.critical_icon`** — an icon (🔥 by default, `""` to disable) shown right before the name of critical alerts.
- **Version in TV mode** — the running version sits next to the TV clock, dimmed to 35% so it stays discreet on a wall display, and becomes fully readable when the HUD bar is hovered.

## [0.6.0] - 2026-09-04

### Changed
- **Link templates are percent-encoded and validated** — values substituted into a `link_template` are now percent-encoded, so a label containing a space, `&` or `/` no longer produces a broken URL. A template referencing a label the alert does not carry is treated as unusable instead of emitting a URL with `{{.Labels.foo}}` left in it: the source link falls back to the generator URL then to `dashboard_url`, and the alert link is simply dropped. Only `http`/`https` links are handed to the frontend, so a `javascript:` URL coming from a source's `generatorURL` can no longer be rendered as a clickable link.
- **Silences are only fetched when something is silenced** — the Alertmanager `/silences` endpoint was queried once per source on every poll just to resolve comments; it is now skipped when no alert reports a silence.
- **User agent reports the real version** — was hardcoded to `alertview/0.1`.

### Added
- **Prefix labels** — `display.prefix_labels` (default `["hostname"]`) shows labels in front of the alert name, joined by `display.prefix_separator` (default `" / "`), in both normal and TV mode. Only the labels the alert carries are rendered, they are shown even when absent from `display.labels`, and they are removed from the trailing label chips so nothing appears twice.
- **TV mode by default** — `display.tv_mode_default` starts the dashboard in TV mode in a browser where nobody has used the TV button yet. An explicit choice still wins and is remembered; `?tv=1` in the URL wins over both, which is the reliable option for a kiosk display.
- **Automatic light/dark theme** — `display.theme` accepts `auto` (now the default), which follows the OS setting and switches live when the OS switches. The theme button cycles `auto → light → dark` and its icon shows the preference rather than the resolved theme. The theme is now resolved by a small inline script before the first paint, so a light-theme user no longer gets a flash of the dark palette, and `<meta name="theme-color">` follows the resolved theme. `display.theme` set to `dark` or `light` finally has an effect — it was documented but silently ignored, only `localStorage` drove the theme. A `theme` holding a URL is still treated as a custom stylesheet, and `display.custom_css` is the explicit way to declare one.
- **Clickable alerts** — `display.alert_link_template` (overridable per source) makes the whole card or TV row clickable, with a URL built from the alert's labels. It is fully independent from the ↗ "open in the source" button, which keeps its own destination and can be hidden with `display.source_link: false` (also overridable per source). `display.link_new_tab: false` keeps links in the same tab for kiosk displays.
- **Configurable severity order** — `display.severity_order` defines the severity ranking (most severe first), used for alert sorting, the filter chips in normal and TV mode, the sound picked for a batch of new alerts and the group severity badges. Defaults to `["critical", "error", "high", "warning", "info", "none"]`. Custom levels can be declared and any severity missing from the list sorts after every listed level; matching is case-insensitive and understands the `crit`/`err`/`warn`/`information` aliases.

### Fixed
- **SSE feedback loop hammering the sources** — new alerts were detected by comparing against the alert cache, which is only written when `cache_ttl_seconds > 0` (disabled by default). With the default config the cache was always empty, so *every* `/api/alerts` call re-announced *every* alert as new over SSE, and the frontend refreshed on each `new_alert` event. With two browser tabs open this became self-sustaining: measured at ~87 requests/second to the upstream Alertmanager instead of one per `refresh_interval`. Announced fingerprints are now tracked per source in the server state, independently of the cache, and a source seen for the first time is primed silently instead of announcing its whole backlog. The frontend now treats an SSE event as a debounced "refresh soon" signal and no longer plays the sound and notification from there, which also removes the duplicate alerting (`fetchAlerts()` already does its own new-alert diff).
- **HTTP 4xx responses were retried** — the retry loop detected the status code by parsing the first whitespace-separated token of the error message, which is always the literal `HTTP`, so the "don't retry on 4xx" branch never ran. A misconfigured source (404, 401, 403) cost 3 retries and ~7s on every refresh; it now fails in ~10ms. The status code is carried by a typed `HttpStatusError` instead of being re-parsed from a string.
- **Per-source `timeout` was capped at 15s** — the shared HTTP client was built with a 15s global timeout, so a source configured with a longer `timeout` was cut off at 15s anyway. The client no longer sets a global timeout (each fetch is already wrapped in the source's own timeout); a 10s connect timeout is kept.
- **`error` severity sorted last** — `error` was not part of the built-in ranking, so alerts carrying it fell below `info` and `none`, got no filter chip and were rendered with the neutral grey style. It is now a first-class level between `critical` and `high`, with its own color in both normal and TV mode.
- **TV mode rows could hide every label** — the two inline label slots were sliced from the configured list *before* checking which labels the alert actually carries, so an alert missing the first two configured labels showed none inline and pushed the rest behind the `+N` toggle. Labels are now filtered by presence first.
- **`display:` section omitted lost every default** — `DisplayConfig` derived `Default`, which serde uses when the whole `display:` block is missing, bypassing the per-field defaults: no card labels and an empty timezone. `Default` now returns the documented values.

## [0.5.7] - 2026-09-04

### Fixed
- **404 on Alertmanager and Grafana sources** — the source `url` was used as the full API path, so a documented base URL (`http://127.0.0.1:9093`) was queried as `/alerts` instead of `/api/v2/alerts` and every fetch returned `HTTP 404`. The API path is built again from the source type: `{url}/api/v2` for Alertmanager, `{url}/api/alertmanager/grafana/api/v2` for Grafana. URLs that already point at the API (with or without a trailing `/alerts`) are kept as-is, so existing workarounds keep working, and a query string such as `?active=true` is preserved on the alerts request.

## [0.5.6] - 2026-06-23

### Changed
- **More visible trigger age** — the "for …" alert age is now rendered in amber (`--c-warning`) and semibold instead of the dim grey, in both normal and TV modes, so the time since the alert fired stands out.

## [0.5.5] - 2026-06-23

### Fixed
- **SSE connection counter leak (HTTP 429)** — the `/events` connection counter was only decremented when the broadcast channel closed, never on a normal client disconnect (tab close, reload, reconnect). The count leaked upward on every page load until it hit the limit and `/events` returned `429 Too Many Requests` permanently. The counter is now an atomic decremented by a RAII guard that runs whenever the stream is dropped, so disconnects can no longer leak slots. A lagging receiver also no longer terminates the stream — missed events are skipped and the connection is kept.

### Changed
- **TV mode: datasource on hover** — the datasource chip is no longer shown inline in TV mode rows; the datasource name now appears in the tooltip of the link button that opens Grafana/Alertmanager/Zabbix.
- **TV mode: label / age order** — labels are now shown before the "for …" trigger age in TV mode rows.

## [0.5.4] - 2026-06-23

### Reverted
- **TV mode row alignment (0.5.3)** — reverted the fixed-column layout of the trailing metadata in TV mode; rows are back to the previous flexible layout.

## [0.5.3] - 2026-06-23

### Changed
- **TV mode row alignment** — in TV mode, the trailing metadata (source, duration, labels, link) is now laid out in fixed columns so it lines up cleanly across rows. The duration column is right-aligned and the label slot is always reserved, so rows without labels no longer shift the source/duration out of alignment.

## [0.5.2] - 2026-06-23

### Added
- **TV mode in URL state** — the TV mode toggle is now reflected in the URL (`?tv=1`) like the search query and severity filter, so a TV view can be bookmarked or shared via a direct link. URL state takes precedence over the locally stored preference on load.

## [0.5.1] - 2026-06-23

### Fixed
- **Grafana/Alertmanager alert links** — alert links now point to the per-alert URL (`generatorURL`, or a configured `link_template`) instead of the static `dashboard_url`. Previously a `dashboard_url` set to the Grafana home page made every alert link to that page rather than the alert itself. The static `dashboard_url` is now only used as a fallback.

### Changed
- **Incremental alert rendering** — the alert list is now reconciled in place on each refresh: only added, removed or modified alerts touch the DOM, instead of rebuilding the whole list. This removes flicker, preserves scroll position, and keeps expanded groups open across refreshes.

## [0.5.0] - 2026-06-23

### Added
- **Configurable severity label per source** — each Alertmanager/Grafana source can set `severity_label` to choose which label is used to classify severity (defaults to `severity`). Useful when alerts carry the level under a different label name (e.g. `priority`).

### Changed
- Severity label lookup is now case-insensitive, so labels like `Severity` or `SEVERITY` are classified correctly.

## [0.4.6] - 2026-06-13

### Added
- **Progressive Web App (PWA) support** — AlertView is now installable on Android (Chrome), iOS (Safari) and desktop
  - Web app manifest (`/manifest.webmanifest`) with standalone display mode and app icons
  - Service worker (`/sw.js`) caching the static app shell; live data (`/api/*`, `/events`) is never cached
  - App icons (192px, 512px, maskable, apple-touch) embedded in the binary
  - Requires a secure context (HTTPS, or `localhost`) for installation
- Comprehensive documentation structure in `docs/` directory
- API documentation (`docs/api.md`)
- Troubleshooting guide (`docs/troubleshooting.md`)
- FAQ (`docs/faq.md`)
- Example configurations (`docs/examples/`)
- Development documentation (`docs/development/`)

### Changed
- Updated README with comprehensive documentation links

### Fixed
- Alertmanager sources no longer fail to load when the silences endpoint is unreachable; the error is logged and alerts are still served without silence comments

## [0.3.0] - 2024-01-15

### Added
- **Log format configuration**: Added `log_format` field to config file (supports "text" and "json")
  - Can be set via config file or `ALERTVIEW_LOG_FORMAT` environment variable
  - Priority: env var > config file > default ("text")
- **Enhanced README**: Added comprehensive documentation including:
  - Table of Contents
  - Configuration examples for each source type
  - Docker and Docker Compose examples
  - Kubernetes deployment guide
  - API documentation
  - Environment Variables section
  - Per-source configuration documentation
  - Link Template Variables documentation
  - Caching configuration section
  - Display Configuration section
  - Updated Features list
  - Tests section

### Changed
- **Configuration precedence**: Clarified that environment variables override config file settings
- **Documentation**: Updated all documentation to reflect new features

### Fixed
- **ConfigMap path reference**: Fixed Kubernetes manifests path in README (manifests are at root, not in `k8s/` directory)
- **API status values**: Fixed API status values documentation in README

## [0.2.0] - 2024-01-10

### Added
- **11 Major Enhancements**:
  1. **Environment Variables**: Added support for:
     - `ALERTVIEW_PORT`
     - `ALERTVIEW_REFRESH_INTERVAL`
     - `ALERTVIEW_CACHE_TTL`
     - `ALERTVIEW_LOG_FORMAT`
  2. **Per-source Timeout**: Added `timeout` field to `Source` struct (default: 15 seconds)
  3. **Structured Logs**: Added JSON log format support via `tracing-subscriber`
  4. **Gzip Compression**: Added compression support using `tower-http` with compression-gzip feature
  5. **Caching**: Added in-memory cache with configurable TTL (per-source and global)
  6. **Retry Logic**: Implemented exponential backoff with configurable:
     - `max_retries` (default: 3)
     - `initial_delay_ms` (default: 1000)
     - `max_delay_ms` (default: 10000)
  7. **Link Templates**: Added `link_template` field with variable substitution:
     - `{{.Labels.x}}` for label values
     - `{{.Annotations.x}}` for annotation values
     - `{{.Source}}` for source name
     - `{{.Id}}` for alert ID
  8. **Sound Notifications**: Added Web Audio API sound notifications in `static/app.js`
     - Configurable via `play_sounds` in display config
     - Different sounds for different severities
  9. **Customizable Theme**: Added `theme` field in `DisplayConfig`:
     - Values: "dark", "light", "auto", or custom CSS URL
  10. **Timezone Support**: Added `timezone` field in `DisplayConfig`:
      - Values: "UTC", "local", or any IANA timezone
  11. **Unit Tests**: Added 13 comprehensive tests:
      - 8 tests in `src/config.rs`
      - 5 tests in `src/alerts.rs`
- **Health Check Endpoint**: Added `/health` route for monitoring
- **Updated config.example**: Added comments and examples for all new configuration options

### Changed
- **Dependencies**: Added `notify`, `notify-debouncer-mini`, `tower-http`, `chrono` with serde feature
- **Dependencies**: Removed `backoff` crate (using custom retry implementation)
- **AppState**: Modified to include `cache` field
- **API Response**: Added `timezone`, `theme`, `play_sounds` to response

### Fixed
- All tests passing (13 tests)
- Configuration validation improved

## [0.1.0] - 2024-01-05

### Added
- **Core Functionality**:
  - Alert fetching from Alertmanager, Grafana, and Zabbix
  - Alert transformation to common format
  - Web UI for displaying alerts
  - Configuration file support (YAML)
  - Command line arguments
- **Auto-Reload Configuration**:
  - File watcher using `notify` and `notify-debouncer-mini`
  - 500ms debounce for file changes
  - Thread-safe config access using `Arc<RwLock<Config>>`
  - Async config loading with `load_async()` method
- **Background Tasks**:
  - Config file watcher (`start_config_watcher()`)
  - Graceful shutdown handling
- **Docker Support**:
  - Dockerfile for building images
  - Multi-arch support (amd64, arm64)
- **Kubernetes Support**:
  - Deployment, Service, and Ingress manifests
  - ConfigMap support
- **Initial Documentation**:
  - README.md with basic information
  - config.example with example configuration

### Changed
- **Architecture**: Moved from synchronous to asynchronous processing
- **Configuration**: Improved config structure with defaults

### Fixed
- Initial release - all core functionality working

## [0.0.1] - 2024-01-01

### Added
- Project initialization
- Basic Rust project structure
- Initial Cargo.toml with dependencies
- Placeholder files for main components

[Unreleased]: https://github.com/frakev/alertview/compare/v0.13.3...HEAD
[0.13.3]: https://github.com/frakev/alertview/compare/v0.13.2...v0.13.3
[0.13.2]: https://github.com/frakev/alertview/compare/v0.13.1...v0.13.2
[0.13.1]: https://github.com/frakev/alertview/compare/v0.13.0...v0.13.1
[0.13.0]: https://github.com/frakev/alertview/compare/v0.12.0...v0.13.0
[0.12.0]: https://github.com/frakev/alertview/compare/v0.11.1...v0.12.0
[0.11.1]: https://github.com/frakev/alertview/compare/v0.11.0...v0.11.1
[0.11.0]: https://github.com/frakev/alertview/compare/v0.10.3...v0.11.0
[0.10.3]: https://github.com/frakev/alertview/compare/v0.10.2...v0.10.3
[0.10.2]: https://github.com/frakev/alertview/compare/v0.10.1...v0.10.2
[0.10.1]: https://github.com/frakev/alertview/compare/v0.10.0...v0.10.1
[0.10.0]: https://github.com/frakev/alertview/compare/v0.9.1...v0.10.0
[0.9.1]: https://github.com/frakev/alertview/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/frakev/alertview/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/frakev/alertview/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/frakev/alertview/compare/v0.7.1...v0.8.0
[0.7.1]: https://github.com/frakev/alertview/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/frakev/alertview/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/frakev/alertview/compare/v0.5.7...v0.6.0
[0.5.7]: https://github.com/frakev/alertview/compare/v0.5.6...v0.5.7
[0.5.6]: https://github.com/frakev/alertview/compare/v0.5.5...v0.5.6
[0.5.5]: https://github.com/frakev/alertview/compare/v0.5.4...v0.5.5
[0.5.4]: https://github.com/frakev/alertview/compare/v0.5.3...v0.5.4
[0.5.3]: https://github.com/frakev/alertview/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/frakev/alertview/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/frakev/alertview/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/frakev/alertview/compare/v0.4.8...v0.5.0
[0.4.6]: https://github.com/frakev/alertview/compare/v0.4.5...v0.4.6
