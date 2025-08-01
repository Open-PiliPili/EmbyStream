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

To learn more about the architecture of a decoupled Emby setup, please refer to [**Wiki**](https://www.google.com/search?q=https://github.com/Open-PiliPili/EmbyStream/wiki).

**Screenshot:**

<div align="center">
 <img src="https://raw.githubusercontent.com/Open-PiliPili/EmbyStream/main/res/imgs/run_log.png"/>
</div>

## PART 2. Features

- **Dual-Mode Support**: Can operate as both a frontend and backend service simultaneously, or as standalone services.
- **Universal Compatibility**: Supports all versions of Emby.
- **Multiple Backend Types**:
    - `disk`: For locally mounted storage.
    - `openlist`: For integration with [OpenList](https://github.com/OpenListTeam/OpenList).
    - `direct_link`: For direct links or CDN streaming.
- **STRM Format Support**: Perfectly compatible with `.strm` files, integrating seamlessly with plugins like [StrmAssistant](https://github.com/sjtuross/StrmAssistant/wiki).
- **Link Encryption**: Secures data transmission with link encryption.
- **Link Expiration Protection**: All generated media links automatically expire after a configurable duration, preventing unauthorized redistribution.
- **User-Agent Filtering**: Includes both allowlist and denylist modes for precise access control.
- **Anti-Reverse Proxy Filtering**: Prevents unauthorized playback by restricting access to a specified host.

## PART 3. Install

### From Cargo Crates
```shell
cargo install embystream
```

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

    **Linux:**
    ```shell
    cp ./target/release/embystream /usr/bin
    ```

    **macOS:**

    ```shell
    cp ./target/release/embystream /usr/local/bin
    ```

### From Docker
You can access the Docker Hub URL below and use Docker to install the image.   
[DockerHub: openpilipili/embystream](https://hub.docker.com/r/openpilipili/embystream)

### From Binaries

You can download pre-compiled binaries for macOS and Linux from the [**GitHub Releases**](https://github.com/Open-PiliPili/EmbyStream/tags) page. Simply unzip the file and add the `embystream` executable to your `$PATH`.

## PART 4. RUN

Create `config.toml` based on one of the two templates below, and modify the contents as needed afterward.   

[frontend.toml](https://github.com/Open-PiliPili/EmbyStream/blob/main/template/config/frontend.toml)   
[backend.toml](https://github.com/Open-PiliPili/EmbyStream/blob/main/template/config/backend.toml)

> 💡 **Note**:
> The dual mode simply requires you to fill in both the frontend and backend configuration sections, and set the `stream_mode` in the template configuration file to `dual`.

### With Binaries
```shell
## Default
/usr/bin/embystream run

## Custom
/usr/bin/embystream run --config "$HOME/.config/embystream/config.toml"
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
  openpilipili/embystream:latest
```

### With Docker Compose Run

Reference: [docker-compose.yaml](https://raw.githubusercontent.com/Open-PiliPili/EmbyStream/main/template/docker/docker-compose.yaml)
```shell
docker-compose pull && docker-compose up -d
```

## PART 5. CLI

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

## PART 6. License

Copyright (c) 2025 open-pilipili.

EmbyStream is licensed under the **[GPL-V3 License](https://www.gnu.org/licenses/gpl-3.0.html)**. 