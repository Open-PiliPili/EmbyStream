import { createApp } from "vue";

import App from "./App.vue";
import router from "./router";
import i18n, { getInitialLocaleSetting, setAppLocale } from "./locales";
import { pinia } from "./stores";

import "./styles/tokens.css";
import "./styles/themes.css";
import "./styles/motion.css";
import "./styles/textures.css";
import "./styles/base.css";

async function bootstrap() {
  await setAppLocale(getInitialLocaleSetting());

  const app = createApp(App);

  app.use(pinia);
  app.use(router);
  app.use(i18n);

  app.mount("#app");

  if ("serviceWorker" in navigator) {
    window.addEventListener("load", () => {
      navigator.serviceWorker.register("/sw.js").catch(() => {});
    });
  }
}

bootstrap();
