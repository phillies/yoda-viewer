# CORS Support for Networked Deployments

> Source: next-features.md §3.5 · Effort: XS–S
> Depends on: features/12 (deploying networked without read-only/token is what CORS makes
> easier — ship them together)

## Problem / clarification

Same-origin deployments (the normal case: WASM bundle served by the same Axum server) need no
CORS. It's needed when the API is consumed from a *different* origin: a dev frontend on
another port hitting a remote backend, scripts/notebooks calling the API from browser
contexts, or a future hosted UI pointing at a user's local server. Also relevant: the
`yoda-ui` `App` component takes an `api_base` prop — cross-origin API use is already
architecturally anticipated.

## Design

- `tower-http` feature `"cors"` (dependency already present).
- Config: `YODA_ALLOWED_ORIGINS` — comma-separated exact origins, or `*`.
  Parse in `yoda-config` (`YoDaSettings.allowed_origins: Vec<String>`, default empty =
  no CORS layer at all → zero behavior change for existing deployments).
- In `build_router`/`build_api_router`:

```rust
if !origins.is_empty() {
    let cors = if origins == ["*"] {
        CorsLayer::new().allow_origin(Any)
    } else {
        CorsLayer::new().allow_origin(
            origins.iter().map(|o| o.parse::<HeaderValue>()).collect::<Result<Vec<_>,_>>()?)
    }
    .allow_methods([Method::GET, Method::PUT, Method::POST])
    .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);  // AUTHORIZATION for features/12 token
    router = router.layer(cors);
}
```

- Layer the **API router only** (`.nest("/api", …)`) — the app shell doesn't need it.
- Invalid origin strings → fail startup with a clear `ConfigError` (don't silently serve
  without CORS the operator asked for).
- `*` + credentials is disallowed by spec; we don't use cookies, and the token (features/12)
  travels in a header, which works with `Any`. Document that `*` on a writable instance is
  reckless: recommend explicit origins or read-only mode. Consider *warning* at startup when
  `allowed_origins == ["*"] && !read_only && auth_token.is_none()`.

## Testing

Axum tests: preflight `OPTIONS /api/labels` with `Origin` + `Access-Control-Request-Method:
PUT` → allow headers present for configured origin, absent for unconfigured origin, absent
entirely when env unset. One test that a normal GET still works with the layer active.

## Risks

None technically; the risk is *social* (people exposing writable instances). The startup
warning and the docs pairing with features/12 address it.
