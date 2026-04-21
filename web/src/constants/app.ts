export const APP_NAME = "EmbyStream";

export const EXTERNAL_LINKS = {
  claudeCode: "https://www.anthropic.com/claude-code",
  codex: "https://openai.com/codex/",
  readme:
    "https://github.com/PiliPili-Team/EmbyStream/blob/main/README.md",
  issueNew: "https://github.com/PiliPili-Team/EmbyStream/issues/new",
} as const;

export const STORAGE_KEYS = {
  consoleSignature: "embystream_console_signature",
  themePreference: "embystream_theme_preference",
  loginArtwork: "embystream_login_artwork",
  locale: "embystream_locale",
  sidebarCollapsed: "embystream_sidebar_collapsed",
  mobileTabIndex: "embystream_mobile_tab_index",
  chineseFont: "embystream_font_zh",
  englishFont: "embystream_font_en",
  codeFont: "embystream_font_code",
  renderFont: "embystream_font_render",
  renderWeight: "embystream_font_weight",
  chineseWeight: "embystream_font_weight_zh",
  englishWeight: "embystream_font_weight_en",
  codeWeight: "embystream_font_weight_code",
} as const;

export const CONSOLE_BRANDING = {
  title: `${APP_NAME}\nReadable config. Calm operations.`,
  hint: "Hint: the wizard stays strict, the logs stay admin-only.",
} as const;

export const UI_LABELS = {
  dialogEyebrow: "Dialog",
} as const;

export const DOC_SNIPPETS = {
  nginxProxyExample: `server {
    listen 80;
    server_name stream.example.com;

    location / {
        proxy_pass http://127.0.0.1:17172;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}`,
} as const;
