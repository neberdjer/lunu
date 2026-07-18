# Lunu

Self-hosted audiobook requests and fulfilment: an Overseerr for audiobooks. Users search and
request a book, an admin approves (or the user is auto-approved), and Lunu finds a release through
Prowlarr, sends it to a download client, and imports the finished files into a structured library.

Audiobookshelf is an optional login provider and library-sync source.

## Status

The backend is feature-complete and ships as a single static binary over SQLite (Postgres
optional). The web frontend is in progress;

## Features

- Requests: multi-user accounts with roles, per-user auto-approve and quotas, duplicate guarding,
  an approval workflow, and issue reporting on delivered books.
- Fulfilment: Prowlarr search with a configurable quality and scoring profile (formats, seeders,
  size, preferred and avoided keywords), grabbed to qBittorrent, Transmission, or SABnzbd, routed
  automatically by the release protocol.
- Library: hardlink-first import (copy fallback) into a naming scheme, plus Audiobookshelf sync
  with title, author, and fuzzy matching for items that carry no ASIN.
- Identity: books are deduplicated by work across editions and metadata sources, so one title is
  one request rather than four.
- Metadata: Audnexus with Audible catalogue search, plus OpenLibrary, Google Books, and Hardcover
  as ISBN-keyed fallbacks, behind a pluggable provider trait with per-provider priority and
  failover.
- Auth: local accounts, optional Audiobookshelf login, OIDC single sign-on, reverse-proxy
  forward-auth, and TOTP or email two-factor authentication.
- Notifications: email, Discord, Slack, ntfy, Apprise, and a generic webhook, each activating when
  its url is configured.
- Operations: a bounded in-memory log viewer with a runtime level toggle, a background job queue,
  and live updates over WebSockets.

## Quick start

Set a master key and start the stack:

```sh
cp .env.example .env
# edit .env: set LUNU_MASTER_KEY to a random value of at least 16 characters
#   openssl rand -base64 32
docker compose up -d
```

Open `http://localhost:8080`. The first account you create becomes the admin.

## Configuration

All bootstrap configuration is environment variables read at startup. Application settings
(Prowlarr, download clients, metadata, notifications, OIDC) are managed at runtime through the
admin API and stored in the database, encrypted at rest where they are secret.

- `LUNU_MASTER_KEY` (required): encrypts secrets at rest, at least 16 characters. Losing it makes
  stored secrets unrecoverable. Generate one with `openssl rand -base64 32`.
- `LUNU_BIND` (image default `0.0.0.0:8080`): address to listen on.
- `LUNU_DATABASE_URL` (image default `sqlite:///data/lunu.db?mode=rwc`): a `sqlite://` or
  `postgres://` url.
- `LUNU_SECURE_COOKIES` (default `true`): marks session cookies Secure. Keep true behind HTTPS.
- `LUNU_URL_BASE` (unset): path prefix when served under a subpath, for example `/lunu`.
- `LUNU_TRUSTED_PROXY_HOPS` (default `0`): number of trusted reverse proxies in front of Lunu, for
  client-ip resolution.
- `LUNU_TRUSTED_CLIENT_IP_HEADER` (unset): header a trusted proxy sets with the real client ip.
- `LUNU_FORWARD_AUTH_HEADER` (unset): header a trusted proxy sets with the authenticated username.
- `LUNU_FORWARD_AUTH_PROXIES` (unset): comma-separated proxy ips allowed to assert the forward-auth
  header.
- `LUNU_WORKERS` (default: cpu count, capped): HTTP worker threads.

Forward-auth authenticates a request only when the direct connection comes from a listed proxy ip;
setting the header without an allowlist refuses to boot, because trusting it otherwise is a full
authentication bypass.

## Postgres

Point `LUNU_DATABASE_URL` at a Postgres instance instead of the default SQLite file:

```
LUNU_DATABASE_URL=postgres://lunu:lunu@postgres:5432/lunu
```

## Reverse proxy

Lunu speaks plain HTTP and expects TLS to terminate at a reverse proxy. Behind one, set
`LUNU_TRUSTED_PROXY_HOPS` so client ips resolve correctly, keep `LUNU_SECURE_COOKIES=true`, and set
`LUNU_URL_BASE` if you mount it under a subpath.

## Community

Questions, help, and development chat happen on Discord: https://discord.gg/DxFhhbr2pm

## License

Lunu is licensed under the GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later). See
the `LICENSE`
