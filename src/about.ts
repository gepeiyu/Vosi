import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { applyLocale, setLocale, t } from "./i18n";

type AppInfo = {
  name: string;
  version: string;
  description: string;
  github_url: string;
};

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing element #${id}`);
  return el as T;
}

async function loadAppInfo() {
  const info = await invoke<AppInfo>("get_app_info");

  byId<HTMLHeadingElement>("app-name").textContent = info.name;
  byId<HTMLParagraphElement>("app-version").textContent =
    `${t("about.version_prefix")} ${info.version}`;
  byId<HTMLParagraphElement>("app-description").textContent = info.description;

  const link = byId<HTMLAnchorElement>("github-link");
  link.textContent = info.github_url;
  link.href = info.github_url;
  link.addEventListener("click", (e) => {
    e.preventDefault();
    openUrl(info.github_url).catch(console.error);
  });
}

async function init() {
  const cfg = await invoke<{ general: { locale: string } }>("get_config");
  setLocale(cfg.general.locale);
  applyLocale();
  await loadAppInfo();
}

window.addEventListener("DOMContentLoaded", () => {
  init().catch(console.error);

  listen("locale-changed", async (event) => {
    setLocale(String(event.payload));
    applyLocale();
    await loadAppInfo();
  }).catch(console.error);
});
