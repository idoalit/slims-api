# Authentication

The SLiMS API uses short-lived HS256 access JWTs and rotating opaque refresh sessions.

## Browser flow

1. `POST /auth/login` verifies the credentials, returns an access token, and sets a refresh cookie.
2. Keep the access token in application memory only and send it as `Authorization: Bearer <token>`.
3. When the access token expires, call `POST /auth/refresh`. The API rotates the refresh token and returns a new access token.
4. Call `POST /auth/logout` to revoke the complete refresh-token family.

The refresh cookie is `HttpOnly`, `SameSite=Strict`, scoped to `/`, and uses the `Secure` flag when `COOKIE_SECURE=true`. Persistent remember-me cookies use the configured refresh lifetime; ordinary sessions receive a browser-session cookie with a shorter server-side expiry.

`POST /auth/refresh` and `POST /auth/logout` require this header as an additional CSRF control:

```http
X-Requested-With: XMLHttpRequest
```

Serve the UI and API behind the same HTTPS origin in production. Cross-origin browser deployments must use an exact CORS allowlist and credentials-enabled requests.

## Login

```http
POST /auth/login HTTP/1.1
Content-Type: application/json

{
  "username": "librarian",
  "password": "secret",
  "remember_me": true
}
```

The response contains a short-lived access token. Authentication responses use `Cache-Control: no-store`.
Each item in `attributes.access` contains `module_id`, the stable `module_name`, `read`, and
`write`. Clients must use `module_name` for permission checks because numeric module IDs can
differ between installations.

```json
{
  "module_id": 1,
  "module_name": "bibliography",
  "read": true,
  "write": false
}
```

## Access-token validation

Access tokens require and validate `sub`, `exp`, `nbf`, `iss`, and `aud`. They also contain `iat` and a random `jti`. The API accepts only HS256 signed with `JWT_SECRET`.

## Refresh-token security

- Refresh validators are generated using the operating system CSPRNG.
- Only SHA-256 validator hashes are stored in MySQL.
- Tokens rotate on every refresh and old-token reuse revokes the entire family.
- A two-second concurrency grace avoids false replay detection from simultaneous browser tabs; it does not return a token for the old session.
- Logout revokes the complete family rather than only deleting the browser cookie.

## Login abuse protection

Failed logins are tracked using a SHA-256 hash of the normalized username. Five failures inside fifteen minutes lock that login key for fifteen minutes. Put an additional IP-aware rate limit at the trusted reverse proxy or API gateway because the application cannot safely infer client IP without deployment-specific trusted-proxy configuration.

## Public endpoints

These endpoints do not require a bearer token:

- `GET /health`
- `POST /auth/login`
- `POST /auth/refresh` (requires the refresh cookie)
- `POST /auth/logout`
- `GET /catalog/biblios`
- `GET /catalog/biblios/search?q={keyword}`
- `GET /catalog/biblios/{biblio_id}`
