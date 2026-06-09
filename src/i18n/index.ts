import type { Locale } from "./types";
import { parseLocale } from "./types";
import zh from "../locales/zh.json";
import en from "../locales/en.json";
import ja from "../locales/ja.json";

const catalogs: Record<Locale, Record<string, string>> = { zh, en, ja };

let currentLocale: Locale = "zh";

export function setLocale(locale: Locale | string): void {
  currentLocale = parseLocale(String(locale));
}

export function getLocale(): Locale {
  return currentLocale;
}

export function t(key: string, vars?: Record<string, string>): string {
  let text =
    catalogs[currentLocale][key] ??
    catalogs.zh[key];
  if (text === undefined) {
    console.warn(`missing i18n key: ${key}`);
    text = key;
  }
  if (vars) {
    for (const [name, value] of Object.entries(vars)) {
      text = text.split(`{${name}}`).join(value);
    }
  }
  return text;
}

export function applyLocale(): void {
  document.documentElement.lang =
    currentLocale === "zh" ? "zh-CN" : currentLocale === "ja" ? "ja" : "en";

  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    const key = el.dataset.i18n!;
    el.textContent = t(key);
  });

  document.querySelectorAll<HTMLOptionElement>("option[data-i18n]").forEach((el) => {
    const key = el.dataset.i18n!;
    el.textContent = t(key);
  });

  const titleKey = document.body.dataset.windowTitle;
  if (titleKey) {
    document.title = t(titleKey);
  }
}
