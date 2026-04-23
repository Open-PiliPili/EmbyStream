/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";

  const component: DefineComponent<Record<string, never>, Record<string, never>, any>;
  export default component;
}

declare const __APP_VERSION__: string;
declare const __APP_GITHUB_URL__: string;
declare const __APP_CHANGELOG_URL__: string;
