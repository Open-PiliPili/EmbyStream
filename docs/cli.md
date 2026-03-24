# CLI usage

The binary name is `embystream`. All subcommands support `--help`.

## Global behavior

```text
embystream [COMMAND]
```

- **`embystream` (no subcommand)** — exits without starting servers. Always invoke **`run`** to start gateways.
- **`embystream --version`** — print version.
- **`embystream --help`** — list commands.

## Language (`--lang`)

| Value | Effect |
|-------|--------|
| `en` (default) | English `--help` text and English prompts in `embystream config …`. |
| `zh` | Simplified Chinese `--help` (top-level descriptions) and Chinese prompts in the configuration wizard. |

Put `--lang` before `--help` if you want localized help, e.g. `embystream --lang zh --help` or `embystream --lang zh config template --help`.

---

## `embystream run`

Starts the configured HTTP(S) gateways according to `[General].stream_mode`:

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

## `embystream config`

Interactive configuration assistant (English prompts).

### `embystream config show`

Scans the **current working directory** for valid TOML configs, lets you pick one, and prints it with secrets masked unless you confirm.

```bash
cd /path/to/configs
embystream config show
```

### `embystream config template`

Walks through `stream_mode` and related choices, then writes a starter `config.toml` via a temporary file and **atomic rename** (safer on live systems).

```bash
cd ~/embystream
embystream config template
```

Use this for a first-time layout; refine paths, tokens, and `[[BackendNode]]` entries afterward.

---

## Related

- [User guide](user-guide.md)
- [Configuration reference](configuration-reference.md)
