# Security Policy

## Reporting Security Vulnerabilities

If you discover a security vulnerability in AlertView, please report it responsibly by emailing the maintainer at [security@frakev.com](mailto:security@frakev.com).

Please do not report security vulnerabilities through public GitHub issues.

## Supported Versions

Only the latest major version receives security updates:

| Version | Supported |
|---------|-----------|
| 0.4.x   | ✅ Yes    |
| < 0.4   | ❌ No     |

## Security Best Practices

### Configuration

- **Use HTTPS**: Always use HTTPS for all source URLs
- **TLS Verification**: Keep `tls_insecure: false` unless absolutely necessary
- **Bearer Tokens**: Use bearer tokens instead of basic auth when possible
- **Minimal Permissions**: Use tokens with read-only permissions

### Deployment

- **Network Isolation**: Deploy AlertView in a private network
- **Authentication**: Add authentication proxy (Nginx, Traefik) in front
- **Rate Limiting**: Configure rate limiting for the API
- **Secrets Management**: Use Kubernetes secrets or vault for tokens

### Monitoring

- Monitor logs for failed authentication attempts
- Set up alerts for unusual activity
- Regularly rotate API tokens

## Vulnerability Disclosure Process

1. **Initial Report**: Security issue reported to maintainer
2. **Acknowledgment**: Maintainer acknowledges receipt within 48 hours
3. **Investigation**: Issue is investigated and confirmed
4. **Patch Development**: Fix is developed and tested
5. **Release**: Security patch is released
6. **Disclosure**: Public disclosure after patch is available

## Security Features

- **No Database**: AlertView doesn't store any data persistently
- **Read-Only**: Only reads alert data, never modifies sources
- **Cache Control**: Configurable cache TTL to limit data retention
- **Input Validation**: All inputs are validated and sanitized

## Responsible Disclosure

We ask that you:
- Give us reasonable time to respond before disclosing publicly
- Provide detailed reproduction steps
- Keep communication confidential until patch is available

Thank you for helping keep AlertView secure!