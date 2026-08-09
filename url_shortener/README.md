# url_shortener

A tiny URL shortener built as a REST API in Rust. Long URLs in, short codes out —
the short code redirects back to the original URL.

Built with **axum** + **tokio**, with an emphasis on tests (unit + integration).

## Features

- `POST /shorten` — create a short link from any `http://` or `https://` URL
- `GET /{code}` — 307-redirect to the stored URL (404 for unknown codes)
- Base-62 short codes (`0`, `1`, ..., `z`, `A`, ..., `Z`, `10`, `11`, ...)
- Input validation: empty or non-http URLs get a `400`
- Configurable port via the `PORT` environment variable
- 9 tests: unit tests for the code encoder, integration tests driving the full HTTP router

## Quick start

```bash
cargo run
# listening on http://0.0.0.0:3000
```

```bash
# 1. shorten a URL
curl -X POST http://localhost:3000/shorten \
  -H 'Content-Type: application/json' \
  -d '{"url":"https://www.rust-lang.org/learn"}'
# {"short":"0"}

# 2. follow the link
curl -i http://localhost:3000/0
# HTTP/1.1 307 Temporary Redirect
# location: https://www.rust-lang.org/learn
```

## API

| Method | Path       | Body                    | Returns                            |
|--------|------------|-------------------------|------------------------------------|
| POST   | `/shorten` | `{"url": "https://..."}` | `200 {"short":"0"}` / `400`       |
| GET    | `/{code}`  | —                       | `307` → original URL / `404`       |

## Tests

```bash
cargo test
```

Runs unit tests (`src/lib.rs`) and integration tests (`tests/api.rs`) that fire real
HTTP requests at the router with `tower::ServiceExt::oneshot` — no server or port needed.

## Docker

```bash
docker build -t url_shortener .
docker run -p 3000:3000 url_shortener
```

## Project layout

```
src/
  lib.rs    # AppState, code encoder, handlers, router builder, unit tests
  main.rs   # thin binary: reads PORT, binds, serves
tests/
  api.rs    # integration tests through the full router
```

## Roadmap

- [ ] SQLite persistence so links survive restarts
- [ ] `GET /stats/{code}` — click counters
- [ ] Deploy to a free host (Render / Railway / Fly.io)
