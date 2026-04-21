# CLI usage

The binary name is `embystream`.
All subcommands support `--help`.

## Global behavior

```text
embystream [COMMAND]
```

- **`embystream` (no subcommand)**:
  exits without starting servers. Always invoke **`run`** to start gateways.
- **`embystream --version`** — print version.
- **`embystream --help`** — list commands.

## Language (`--lang`)

| Value | Effect |
|-------|--------|
| `en` (default) | English `--help` text and English prompts in `embystream config …`. |
| `zh` | Simplified Chinese `--help` (top-level descriptions) and Chinese prompts in the configuration wizard. |

Put `--lang` before `--help` if you want localized help, e.g.
`embystream --lang zh --help` or
`embystream --lang zh config template --help`.

---

## `embystream run`

Starts the configured HTTP(S) gateways according to
`[General].stream_mode`:

| Mode       | What starts |
|------------|-------------|
| `frontend` | Frontend reverse proxy only (HTTP). |
| `backend`  | Backend stream gateway only (HTTPS with `[Http2]` certs). |
| `dual`     | Both; ports must differ. |

### Options

| Option | Description |
|--------|-------------|
| `-c, --config <FILE>` | Path to `config.toml`. If omitted, uses the default discovery order (see [Configuration reference](configuration-reference.md)). |
| `--ssl-cert-file <FILE>` | Override `[Http2].ssl_cert_file` for this process. |
| `--ssl-key-file <FILE>` | Override `[Http2].ssl_key_file` for this process. |

### Examples

```bash
# Default config location
embystream run

# Custom config
embystream run --config /etc/embystream/config.toml

# Temporary TLS files (e.g. renewed certificates)
embystream run -c ./config.toml \
  --ssl-cert-file /run/secrets/cert.pem \
  --ssl-key-file /run/secrets/key.pem
```

---

## `embystream auth google`

Starts a Google OAuth installed-app flow for Drive readonly access.

```bash
embystream auth google \
  --client-id YOUR_CLIENT_ID \
  --secret YOUR_CLIENT_SECRET
```

Behavior:

- Prints the authorization URL every time.
- Tries to open a browser by default.
- Spins up a localhost callback for the installed-app redirect flow.
- Prints `access_token`, `refresh_token`, and `expires_at` after success.

Use `--no-browser` on headless hosts:

```bash
embystream auth google \
  --client-id YOUR_CLIENT_ID \
  --secret YOUR_CLIENT_SECRET \
  --no-browser
```

This still requires completing authorization in a browser that can finish
the installed-app redirect flow.

---

## `embystream config`

Interactive configuration assistant (English prompts).

### `embystream config show`

Scans the **current working directory** for valid TOML configs, lets you
pick one, and prints it with secrets masked unless you confirm.

```bash
cd /path/to/configs
embystream config show
```

### `embystream config template`

Walks through `stream_mode` and related choices, then writes a starter
`config.toml` via a temporary file and **atomic rename**.
This is safer on live systems.

```bash
cd ~/embystream
embystream config template
```

Use this for a first-time layout; refine paths, tokens, and `[[BackendNode]]` entries afterward.

---

## `embystream web`

Starts the Web Config Studio or performs web admin tasks.

### `embystream web serve`

```bash
embystream web serve \
  --listen 127.0.0.1:17172 \
  --data-dir ./web_data \
  --runtime-log-dir ./logs
```

Options:

| Option | Description |
|--------|-------------|
| `--listen <ADDR>` | Web service listen address. Default `127.0.0.1:17172`. |
| `--data-dir <DIR>` | SQLite, sessions, generated artifacts, and audit-log state. Default `./web_data`. |
| `--runtime-log-dir <DIR>` | Runtime log source for the admin log page. Default `./logs`. |
| `--tmdb-api-key <KEY>` | Optional TMDB API key for trending login backgrounds. |

Behavior:

- serves the Rust JSON API
- serves built frontend assets from `web/dist`
- embeds frontend assets into the binary when `web/dist` exists at build time
- falls back to Bing login backgrounds when TMDB is not configured

### `embystream web admin reset-password`

```bash
embystream web admin reset-password \
  --data-dir ./web_data \
  --username admin
```

Behavior:

- resets the target admin password
- prints the new random password once to stdout
- does not expose browser-based password recovery

---

## Related

- [User guide](user-guide.md)
- [Configuration reference](configuration-reference.md)
- [Google OAuth Desktop App Setup (EN)](google-oauth-desktop-app-setup.en.md)
- [Google OAuth Desktop App 创建教程 (ZH-CN)](google-oauth-desktop-app-setup.zh-CN.md)
