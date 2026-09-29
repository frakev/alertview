# Security Policy

## Reporting a Vulnerability

Use GitHub's **private vulnerability reporting**: go to the
[Security tab](https://github.com/frakev/alertview/security) and click
*Report a vulnerability*. It opens an advisory only you and the maintainer can
see, which is the right place for a report that comes with a working
reproduction.

If that form is not available to you, open a
[public issue](https://github.com/frakev/alertview/issues) instead — but keep
it to what is needed to identify the problem, and leave out anything that
would work as a ready-made exploit until a fix is out.

AlertView is maintained on personal time: expect a reply in days, not hours.
You will get an acknowledgement, an assessment, and a fix or an explanation of
why it is not one.

## Supported Versions

Only the latest release receives fixes. AlertView is pre-1.0 and moves in
minor versions; there are no maintenance branches.

| Version | Supported |
|---------|-----------|
| 0.13.x  | ✅ Yes    |
| < 0.13  | ❌ No — upgrade |

Dependencies are checked against the RustSec advisory database on every push
and pull request (`cargo audit` in CI); an advisory fails the build.

## What AlertView Assumes About Its Environment

**AlertView has no authentication of its own and binds to `0.0.0.0`.** Anyone
who can reach the port can read every alert it aggregates — hostnames, labels,
messages — through the dashboard or through `/api/alerts`. That is by design:
it is meant to sit behind something that authenticates.

Consequently, the following are **not** treated as vulnerabilities:

- Reaching `/api/alerts` without credentials.
- The absence of rate limiting, CSRF tokens or session handling.
- Anything that requires write access to the configuration file, which is
  trusted input: it holds the source credentials in the first place.
- Exhausting the live-update stream. `/events` is capped at 100 concurrent
  connections **in total**, not per client, so one client can take them all.
  The dashboard degrades to polling rather than breaking, and per-client limits
  belong at the proxy — where the client's identity is actually known.

These **are** treated as vulnerabilities:

- Anything an *alert* can do to a viewer — a label, an annotation, a silence
  comment or a severity that reaches the browser as markup or script rather
  than as text. Alert content comes from outside and is not trusted.
- Credentials from the configuration leaking into a response, a log line or a
  link handed to the browser.
- A link built from alert content that navigates somewhere other than the
  `http(s)` URL it appears to be.
- Anything that lets a source's response affect the server beyond its own
  entry in the dashboard.

## Deployment Guidance

### Configuration

- **Use HTTPS** for every source URL.
- Keep `tls_insecure: false` unless you have no other option — it disables
  certificate verification for *all* sources at once.
- Prefer a bearer token over basic auth, with **read-only** permissions.
- Never commit `config.yaml`: it holds credentials, and it is gitignored for
  that reason. On Kubernetes, put it in a Secret rather than a ConfigMap —
  see [the deployment guide](docs/deployment.md#credentials).

### Deployment

- **Put a reverse proxy in front** and authenticate there (see
  [Reverse proxy](docs/deployment.md#reverse-proxy)). This is the one thing
  that matters most.
- Deploy on a private network; do not publish the port directly.
- Rate limit at the proxy, where it belongs.
- Run the container as shipped: non-root (`65532`), read-only root filesystem,
  all capabilities dropped.

### Operations

- Rotate source tokens periodically.
- Watch the logs for repeated fetch failures — a source answering `401` usually
  means a token was revoked or rotated out from under you.

## What AlertView Does On Its Own

- **Stores nothing.** No database, no files written, no state that survives a
  restart. The result of the last poll is kept in memory, and replaced by the
  next one.
- **Never writes to a source.** Every call is a read.
- **Polls its sources on its own schedule.** A request to `/api/alerts` is
  served from the last poll and never reaches a source, so an unauthenticated
  caller cannot amplify one cheap request into many against your monitoring
  systems.
- **Sends a Content-Security-Policy** with `script-src 'self'` and no
  `unsafe-inline`, which is what stops an injected event handler from running.
  The only outside host it allows is the custom stylesheet's, when one is
  configured (`display.custom_css`, or a `theme` holding a URL), for that
  stylesheet and the fonts and images it loads.
- **Escapes alert content** before it reaches the page, in text and in
  attributes alike, and only ever hands the browser `http(s)` links —
  a `javascript:` generator URL is dropped, and values substituted into a link
  template are percent-encoded.
- **Redacts credentials** found in a source URL before an error message
  reaches the API or the logs.

## Disclosure

Report privately, give a reasonable window for a fix, and we will credit you
in the release notes unless you would rather we did not. Fixed vulnerabilities
are described in the [CHANGELOG](CHANGELOG.md) once a release carrying the fix
is out.
