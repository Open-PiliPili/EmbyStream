# EmbyStream

<p align="center">
<a href="https://github.com/Open-PiliPili/EmbyStream">
<img alt="EmbyStream Logo" src="https://raw.githubusercontent.com/Open-PiliPili/EmbyStream/main/res/imgs/logo.jpg" width="400" />
</a>
</p>
<h1 align="center">EmbyStream</h1>
<p align="center">
A highly customizable Emby streaming proxy (frontend / backend split), written in Rust.
</p>
<p align="center">
<a href="https://t.me/openpilipili_chat"><img src="https://img.shields.io/badge/-Telegram_Group-red?color=blue&logo=telegram&logoColor=white" alt="Telegram"></a>
<a href="https://github.com/open-pilipili/EmbyStream/commit/main"><img src="https://img.shields.io/github/commit-activity/m/open-pilipili/EmbyStream/main" alt="Commit Activity"></a>
<a href="https://github.com/open-pilipili/EmbyStream"><img src="https://img.shields.io/github/languages/top/open-pilipili/EmbyStream" alt="Top Language"></a>
<a href="https://github.com/open-pilipili/EmbyStream/blob/main/LICENSE"><img src="https://img.shields.io/github/license/open-pilipili/EmbyStream" alt="Github License"></a>
<a href="https://github.com/Open-PiliPili/EmbyStream/actions/workflows/ci.yaml"><img src="https://github.com/Open-PiliPili/EmbyStream/actions/workflows/ci.yaml/badge.svg" alt="Linux CI"></a> <a href="https://github.com/open-pilipili/EmbyStream/wiki"><img src="https://img.shields.io/badge/-Wiki-red?color=blue&logo=github&logoColor=white" alt="Wiki"></a>
</p>

## About

EmbyStream is a reverse proxy and stream gateway for Emby: a **frontend** gateway talks to Emby and rewrites traffic; a **backend** gateway serves or redirects media with signed, expiring links. You can run either side alone or both in **dual** mode (two ports). The stack is async Rust (Hyper / Tokio) with optional TLS on the backend listener.

Architecture background: [**EmbyStream Wiki**](https://github.com/Open-PiliPili/EmbyStream/wiki).

**Screenshot:**

<div align="center">
 <img src="https://raw.githubusercontent.com/Open-PiliPili/EmbyStream/main/res/imgs/run_log.png"/>
</div>

## Features (overview)

- **Stream modes:** `frontend`, `backend`, or `dual` (distinct ports required in dual).
- **Storage drivers** (`[[BackendNode]].type`): **Disk**, **OpenList**, **DirectLink**, **WebDav**, plus **StreamRelay** for chaining gateways without decrypting `sign`.
- **STRM-friendly** paths, path rewrite rules, and optional **fallback** video when a file is missing.
- **Signed / encrypted playback URLs** with expiry; **per-node** redirect vs proxy and **per-device** speed limits.
- **User-Agent** allow/deny; **anti–reverse-proxy** host checks on the frontend (and per node where configured).
- **Interactive config wizard:** `embystream config template` / `config show`; use **`--lang zh`** for Simplified Chinese prompts and localized `--help` (default `en`).
- **CORS / OPTIONS**, playlist handling, API response caching on the forward path, and **HTTP/2 TLS** for the backend via `[Http2]` or CLI overrides.

Detailed behavior and every TOML field: **[Configuration reference](docs/configuration-reference.md)**.

## Documentation

| Document | Description |
|----------|-------------|
| [User guide](docs/user-guide.md) | Deployment patterns, security notes, Docker notes, first-time setup |
| [Configuration reference](docs/configuration-reference.md) | All config sections, scenarios, and examples (English) |
| [CLI usage](docs/cli.md) | `run`, `config`, flags |

## Install

### From crates.io

```shell
cargo install embystream
```

### From source

```shell
git clone https://github.com/Open-PiliPili/EmbyStream.git
cd EmbyStream && cargo build --release
```

Install the binary (example paths):

- **Linux:** `cp ./target/release/embystream /usr/local/bin/`
- **macOS:** `cp ./target/release/embystream /usr/local/bin/`

### Docker

Image: [Docker Hub — openpilipili/embystream](https://hub.docker.com/r/openpilipili/embystream). Mount your `config.toml` and publish the ports that match your config (the bundled template listens on **60001** / **60002**).

### Prebuilt binaries

See [**GitHub Releases**](https://github.com/Open-PiliPili/EmbyStream/releases).

## Run

1. Start from the template [`src/config/config.toml.template`](src/config/config.toml.template) or run `embystream config template`.
2. Adjust `[Emby]`, `[[BackendNode]]`, and ports for your layout (see [User guide](docs/user-guide.md)).
3. Start the service:

```shell
embystream run
embystream run --config "$HOME/.config/embystream/config.toml"
```

**Docker (example):** map host port to the **same** port as `listen_port` in your config.

```shell
docker run -d \
  --name ${CONTAINER_NAME:-embystream} \
  -p 60001:60001 \
  -e TZ="Asia/Shanghai" \
  -v ./config/config.toml:/config/embystream/config.toml \
  --log-driver json-file \
  --log-opt max-size=50m \
  --log-opt max-file=3 \
  --restart unless-stopped \
  openpilipili/embystream:latest
```

Compose: [`template/docker/docker-compose.yaml`](template/docker/docker-compose.yaml) (update published ports if you change `listen_port`).

> **Note:** Examples that set `PUID` / `PGID` / `privileged` are optional host policies; the minimal image does not map PUID/PGID internally.

## CLI (summary)

Use **`embystream run`** to start gateways. Pass **`--lang zh`** (global) for Chinese wizard text and Chinese top-level `--help`. Details: **[CLI usage](docs/cli.md)**.

## License

Copyright (c) 2025 open-pilipili.

EmbyStream is licensed under the **[GPL-3.0](https://www.gnu.org/licenses/gpl-3.0.html)**.
