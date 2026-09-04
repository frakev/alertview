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
| 0.10.x  | ✅ Yes    |
| < 0.10  | ❌ No — upgrade |

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
  see [the deployment guide](docs/deployment/kubernetes.md#secrets-management).

### Deployment

- **Put a reverse proxy in front** and authenticate there (see
  [Reverse proxy](docs/deployment/reverse-proxy.md)). This is the one thing
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
  restart. The optional cache is in memory and bounded by `cache_ttl_seconds`.
- **Never writes to a source.** Every call is a read.
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
