# User guide

EmbyStream sits between **clients** and **Emby** in a split “frontend / backend” design: the frontend speaks to Emby and rewrites API responses; the backend serves (or redirects to) real media using signed URLs and optional storage drivers.

---

## When to use which `stream_mode`

| Scenario | Suggested mode |
|----------|----------------|
| You only want a reverse proxy in front of Emby (TLS termination, UA filter, path rewrite, anti-hotlink) | `frontend` |
| You run a dedicated media edge node that must not talk to Emby directly | `backend` |
| One process handles both proxy and streaming on two ports | `dual` |

In `dual` mode, assign **two different ports** — for example frontend `60001` and backend `60002` — or startup will fail with a port conflict error.

---

## Typical network layout

1. **Emby** runs on your LAN (e.g. `http://127.0.0.1:8096`).
2. **Frontend** listens on a port you expose to users; `[Emby]` points at the real server.
3. **Backend** listens on HTTPS; Emby library paths in `.strm` or virtual paths should map to `[[BackendNode]]` patterns so the correct driver handles each file.

Point Emby’s **external domain / streaming** settings at your **frontend** public URL where your workflow requires it, and ensure signed URLs reference the **backend** public `[Backend]` URL (`base_url`, `port`, `path`).

---

## STRM and plugins

`.strm` files and tools such as [StrmAssistant](https://github.com/sjtuross/StrmAssistant/wiki) are supported: paths inside streams should match your `[[BackendNode]]` patterns (and optional per-node path rewrites).

---

## Security checklist

- Rotate **`encipher_key`** and **`encipher_iv`** from the template before production.
- Use **`[UserAgent]`** and **`[Frontend.AntiReverseProxy]`** (or per-node anti-reverse-proxy) if you expose the service to the internet.
- Terminate TLS on the **backend** listener via `[Http2]` (or a reverse proxy that forwards to the backend port with correct headers).
- Prefer **`embystream config show`** when sharing configs — it masks secrets by default.

---

## Docker quick start

1. Mount your real `config.toml` over `/config/embystream/config.toml`.
2. Publish the ports that match **`listen_port`** in that file (the stock template uses **60001** for frontend and **60002** for backend).
3. Environment variables such as `PUID` / `PGID` in example compose files are **not** consumed by the minimal Alpine image; run the container as the user you need, or adjust file ownership on the host.

See the [README](../README.md) Docker section and [`template/docker/docker-compose.yaml`](../template/docker/docker-compose.yaml).

---

## First-time configuration

1. Copy [`src/config/config.toml.template`](../src/config/config.toml.template) or run **`embystream config template`** (see [CLI](cli.md)).
2. Set `[Emby]` and at least one **`[[BackendNode]]`** appropriate for your storage.
3. Run **`embystream run`** and fix any validation errors (regex, missing sections for `stream_mode`).

Full field documentation: [Configuration reference](configuration-reference.md).

If you are configuring a `googleDrive` backend for the first time, create the
Google OAuth `Desktop app` client first:

- [Google OAuth Desktop App Setup](google-oauth-desktop-app-setup.en.md)
- [Google OAuth Desktop App 创建教程](google-oauth-desktop-app-setup.zh-CN.md)

---

## Getting help

- **Wiki:** [EmbyStream Wiki](https://github.com/Open-PiliPili/EmbyStream/wiki)
- **Community:** Telegram link in the main [README](../README.md)
