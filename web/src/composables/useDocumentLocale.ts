import { watchEffect } from "vue";
import { useI18n } from "vue-i18n";

export function useDocumentLocale() {
  const { locale } = useI18n();

  watchEffect(() => {
    document.documentElement.lang = locale.value;
  });
}
