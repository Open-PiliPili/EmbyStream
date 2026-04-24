# User guide

EmbyStream is now primarily operated through the built-in Web Config Studio. The browser flow is the recommended way to create configs, inspect generated artifacts, and review admin logs. Direct `config.toml` editing and `embystream run` remain available as a fallback.

---

## Recommended setup flow

1. Build the frontend assets under [`web/`](../web/).
2. Start `embystream web serve`.
3. Open the Web Config Studio in a browser.
4. Create or sign in to a local admin account.
5. Generate artifacts and deploy the produced `config.toml` to the host that will run the gateway.

Quick start:

```bash
cd web
bun install
bun run build
cd ..

cargo run -- web serve \
  --listen 127.0.0.1:6888 \
  --data-dir ./web_data \
  --runtime-log-dir ./logs
```

The web admin provides:

- draft autosave and restore
- generated `config.toml`, `nginx.conf`, `docker-compose.yaml`, `systemd.service`, and `pm2.config.cjs`
- admin-only runtime, stream, and audit log viewing

For bundled local builds, use [`scripts/build-binary.sh`](../scripts/build-binary.sh).
For container images, use [`scripts/build-docker.sh`](../scripts/build-docker.sh).

---

## When to use which `stream_mode`

| Scenario | Suggested mode |
|----------|----------------|
| You only need a reverse proxy in front of Emby | `frontend` |
| You run a dedicated media edge node that should not talk to Emby directly | `backend` |
| One process should host both proxy and stream entry points | `dual` |

In `dual` mode, frontend and backend must use different ports.

---

## Typical network layout

1. Emby runs on your LAN, for example `http://127.0.0.1:8096`.
2. The frontend listens on the public entry port and talks to Emby.
3. The backend serves or redirects media using signed URLs and your configured `[[BackendNode]]` rules.

Point Emby external playback at the frontend URL when your workflow needs it, and ensure the backend public `[Backend]` URL is reachable by clients.

---

## Packaging paths

### Embedded local binary

`build.rs` embeds `web/dist` into the Rust binary when those frontend assets exist at compile time.

```bash
./scripts/build-binary.sh
```

By default the script writes the copied binary and optional `.tar.gz` package to `./.build/binary/release/` or `./.build/binary/debug/`.

### Docker image

Build the integrated Docker image:

```bash
./scripts/build-docker.sh --tag embystream:latest
```
When the image is loaded locally, the script also exports a Docker image tar and metadata under `./.build/docker/`.

---

## Security checklist

- Replace `encipher_key` and `encipher_iv` from the template before internet exposure.
- Use `[UserAgent]` and `[Frontend.AntiReverseProxy]` when exposing the frontend publicly.
- Protect the backend with TLS through `[Http2]` or a correctly configured reverse proxy.
- Prefer `embystream config show` when sharing configs, because secrets are masked by default.
- Restrict access to the web admin endpoint; only `admin` can access browser logs and user management.

---

## Browser-specific behavior

- `embystream web serve --tmdb-api-key ...` enables TMDB trending backgrounds on the login page.
- Without TMDB, the login page falls back to Bing daily images.
- Background metadata is cached for at least six hours.
- Admin logs show runtime, stream, and audit entries newest first.
- Token-like segments are masked before browser output.

---

## CLI fallback

Use the CLI path only when you intentionally want to manage the gateway without the web admin.

1. Start from [`src/config/config.toml.template`](../src/config/config.toml.template) or run `embystream config template`.
2. Fill `[Emby]`, `[General]`, and the required `[Frontend]` / `[Backend]` sections for your chosen `stream_mode`.
3. Add at least one matching `[[BackendNode]]`.
4. Start the gateway with `embystream run`.

```bash
embystream config template
embystream run --config ./config.toml
```

If you need Google Drive credentials first:

- [Google OAuth Desktop App Setup](google-oauth-desktop-app-setup.en.md)
- [Google OAuth Desktop App 创建教程](google-oauth-desktop-app-setup.zh-CN.md)

Full field details are in the [Configuration reference](configuration-reference.md).

---

## Getting help

- [README](../README.md)
- [CLI usage](cli.md)
- [Configuration reference](configuration-reference.md)
- [EmbyStream Wiki](https://github.com/Open-PiliPili/EmbyStream/wiki)
