# EmbyStream

<p align="center">
<a href="https://github.com/Open-PiliPili/EmbyStream">
<img alt="EmbyStream Logo" src="https://raw.githubusercontent.com/Open-PiliPili/EmbyStream/main/res/imgs/logo.jpg" width="400" />
</a>
</p>
<h1 align="center">EmbyStream</h1>
<p align="center">
A highly customizable, decoupled frontend/backend application for Emby, written entirely in Rust.
</p>
<p align="center">
<a href="https://t.me/openpilipili_chat"><img src="https://img.shields.io/badge/-Telegram_Group-red?color=blue&logo=telegram&logoColor=white" alt="Telegram"></a>
<a href="https://github.com/open-pilipili/EmbyStream/commit/main"><img src="https://img.shields.io/github/commit-activity/m/open-pilipili/EmbyStream/main" alt="Commit Activity"></a>
<a href="https://github.com/open-pilipili/EmbyStream"><img src="https://img.shields.io/github/languages/top/open-pilipili/EmbyStream" alt="Top Language"></a>
<a href="https://github.com/open-pilipili/EmbyStream/blob/main/LICENSE"><img src="https://img.shields.io/github/license/open-pilipili/EmbyStream" alt="Github License"></a>
<a href="https://github.com/Open-PiliPili/EmbyStream/actions/workflows/ci.yaml"><img src="https://github.com/Open-PiliPili/EmbyStream/actions/workflows/ci.yaml/badge.svg" alt="Linux CI"></a> <a href="https://github.com/open-pilipili/EmbyStream/wiki"><img src="https://img.shields.io/badge/-Wiki-red?color=blue&logo=github&logoColor=white" alt="Wiki"></a>
</p>

## PART 1. About

EmbyStream is a highly customizable, decoupled frontend/backend application for Emby. It is written entirely in Rust for ultimate performance and memory safety.

To learn more about the architecture of a decoupled Emby setup, please refer to our [**Wiki**](https://www.google.com/search?q=https://github.com/Open-PiliPili/EmbyStream/wiki).

**Screenshot:**

<div align="center">
 <img src="https://raw.githubusercontent.com/Open-PiliPili/EmbyStream/main/res/imgs/run_log.png"/>
</div>

## PART 2. Features

- **Dual-Mode Support**: Can operate as both a frontend and backend service simultaneously, or as standalone services.
- **Universal Compatibility**: Supports all versions of Emby.
- **Multiple Backend Types**:
    - `disk`: For locally mounted storage.
    - `openlist` (beta): For integration with OpenList.
    - `direct_link` (beta): For direct links or CDN streaming.
- **STRM Format Support**: Perfectly compatible with `.strm` files, integrating seamlessly with plugins like "[StrmAssistant](https://github.com/sjtuross/StrmAssistant/wiki)".
- **Link Encryption**: Secures data transmission with link encryption.
- **User-Agent Filtering**: Includes both allowlist and denylist modes for precise access control.
- **Anti-Reverse Proxy Filtering** (beta): Prevents unauthorized playback by restricting access to a specified host.

## PART 3. Install

### From Source (with Cargo)

1. Clone the repository:

   ```shell
   git clone https://github.com/Open-PiliPili/EmbyStream.git
   ```

2. Enter the directory and build the project:

   ```shell
   cd EmbyStream && cargo build --release
   ```

3. Copy the compiled binary to your system's PATH:

    - **Linux:**

      ```shell
      cp ./target/release/embystream /usr/bin
      ```

    - **macOS:**

      ```shell
      cp ./target/release/embystream /usr/local/bin
      ```

### With Docker Run

```shell
docker run -d \
  --name ${CONTAINER_NAME:-embystream} \
  -p 50001:50001 \
  -e TZ="Asia/Shanghai" \
  -e PUID=1000 \
  -e PGID=1000 \
  -e UMASK=022 \
  -v ./config/config.toml:/config/embystream/config.toml \
  --privileged \
  --log-driver json-file \
  --log-opt max-size=50m \
  --log-opt max-file=3 \
  --restart unless-stopped \
  embystream
```

### With docker-compose

Reference: [docker-compose.yam](https://raw.githubusercontent.com/Open-PiliPili/EmbyStream/main/template/docker/docker-compose.yaml)
```shell
docker-compose pull && docker-compose up -d
```

### Binaries

You can download pre-compiled binaries for macOS and Linux from the [**GitHub Releases**](https://github.com/Open-PiliPili/EmbyStream/tags) page. Simply unzip the file and add the `embystream` executable to your `$PATH`.

## PART 4. CLI

```shell
Another Emby streaming application (frontend/backend separation) written in Rust.

Usage: embystream [COMMAND]

Commands:
  run   
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## PART 5. Configuration File

EmbyStream is configured via a `backend.toml` file. Below is a detailed breakdown of all configuration options.

### `[General]`

General application settings.

| Parameter         | Description                                                  | Default Value        |
| ----------------- | ------------------------------------------------------------ | -------------------- |
| `log_level`       | Application log level. Options: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`. | `"info"`             |
| `memory_mode`     | Memory usage profile. `low`: Minimal memory, reduced performance. `middle`: Balanced (recommended). `high`: Maximum performance, higher memory usage. | `"middle"`           |
| `expired_seconds` | Cache expiration time in seconds. Set to `0` to disable. Recommended range: 300-86400. | `3600`               |
| `stream_mode`     | Service running mode. `frontend`, `backend`, or `dual`.      | `"backend"`          |
| `backend_type`    | Media streaming backend. Options: `"disk"`, `"direct_link"`, `"openlist"`. | `"disk"`             |
| `encipher_key`    | Encryption key (6-16 bytes). **Must be changed in production** and must be identical to the frontend's key. | `"Q4eCbawEp3sCvDvx"` |
| `encipher_iv`     | Encryption IV (6-16 bytes). **Must be changed in production** and must be identical to the frontend's IV. | `"a3cH2abhxnu9hGo5"` |
| `emby_url`        | Base URL of your Emby server (optional).                     | `"http://127.0.0.1"` |
| `emby_port`       | Port for your Emby server (optional).                        | `"8096"`             |
| `emby_api_key`    | API key for Emby (optional, leave empty if not used).        | `"nmbp7mp...a_key"`  |

### `[UserAgent]`

User-Agent filtering settings.

| Parameter  | Description                                                  | Default Value           |
| ---------- | ------------------------------------------------------------ | ----------------------- |
| `mode`     | Filtering mode. `allow`: Allowlist mode. `deny`: Denylist mode. `none`: Disabled. | `"deny"`                |
| `allow_ua` | List of allowed User-Agents (used when `mode = "allow"`).    | `[]`                    |
| `deny_ua`  | List of denied User-Agents (used when `mode = "deny"`).      | `["curl", "wget", ...]` |

### `[Http2]`

Settings for HTTPS/HTTP2 connections.

| Parameter       | Description                                                  | Default Value |
| --------------- | ------------------------------------------------------------ | ------------- |
| `ssl_cert_file` | Path to your SSL/TLS certificate file (PEM format). Required for HTTPS/HTTP2. | `""`          |
| `ssl_key_file`  | Path to your SSL/TLS private key file (PEM format). Required for HTTPS/HTTP2. | `""`          |

### `[Frontend]`

Configuration for the frontend service.

| Parameter     | Description                                               | Default Value |
| ------------- | --------------------------------------------------------- | ------------- |
| `listen_port` | Listening port for the frontend web interface (optional). | `60001`       |

#### `[Frontend.PathRewrite]`

| Parameter     | Description                             | Default Value |
| ------------- | --------------------------------------- | ------------- |
| `enable`      | Set to `true` to enable path rewriting. | `false`       |
| `pattern`     | The regex pattern to search for.        | `""`          |
| `replacement` | The replacement string.                 | `""`          |

#### `[Frontend.AntiReverseProxy]`

| Parameter | Description                                           | Default Value |
| --------- | ----------------------------------------------------- | ------------- |
| `enable`  | Set to `true` to enable anti-reverse proxy filtering. | `false`       |
| `host`    | The allowed host header.                              | `""`          |

### `[Backend]`

Configuration for the backend service.

| Parameter     | Description                                                  | Default Value |
| ------------- | ------------------------------------------------------------ | ------------- |
| `listen_port` | Listening port for the backend stream service.               | `60001`       |
| `base_url`    | Base URL for the stream service.                             | `""`          |
| `path`        | Path component for stream URLs.                              | `"stream"`    |
| `port`        | HTTPS port for the stream service.                           | `"443"`       |
| `proxy_mode`  | Backend proxy mode. `proxy`: Proxies the stream. `redirect`: Redirects to the source. | `"proxy"`     |

#### `[Backend.PathRewrite]`

| Parameter     | Description                                                  | Default Value |
| ------------- | ------------------------------------------------------------ | ------------- |
| `enable`      | Set to `true` to enable path rewriting.                      | `false`       |
| `pattern`     | The regex pattern to search for.                             | `""`          |
| `replacement` | The replacement string. Use `$1` for the first captured group. | `""`          |

#### `[Backend.AntiReverseProxy]`

| Parameter | Description                                           | Default Value |
| --------- | ----------------------------------------------------- | ------------- |
| `enable`  | Set to `true` to enable anti-reverse proxy filtering. | `false`       |
| `host`    | The allowed host header.                              | `""`          |

### `[Disk]`

Configuration for the `disk` backend type.

| Parameter     | Description                                                  | Default Value |
| ------------- | ------------------------------------------------------------ | ------------- |
| `description` | A description for the disk backend (currently for informational purposes). | `""`          |

### `[OpenList]`

Configuration for the `openlist` backend type.

| Parameter    | Description                                          | Default Value |
| ------------ | ---------------------------------------------------- | ------------- |
| `base_url`   | The URL of the OpenList server.                      | `""`          |
| `port`       | The port of the OpenList server.                     | `""`          |
| `token`      | The authentication token for OpenList.               | `""`          |
| `user_agent` | A custom User-Agent to use for requests to OpenList. | `""`          |

### `[DirectLink]`

Configuration for the `direct_link` backend type.

| Parameter    | Description                              | Default Value |
| ------------ | ---------------------------------------- | ------------- |
| `user_agent` | A custom User-Agent to use for requests. | `""`          |

## PART 6. License

Copyright (c) 2025 open-pilipili.

EmbyStream is licensed under the **GPL-V3 License**. See the [official GPL-V3 license text](https://www.gnu.org/licenses/gpl-3.0.html) for more details.