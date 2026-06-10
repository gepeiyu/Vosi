import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { applyLocale, setLocale, t } from "./i18n";

type AppConfig = {
  hotkey: { trigger_key: string; mode: string };
  audio: {
    sample_rate: number;
    silence_threshold_ms: number;
    min_speech_ms: number;
  };
  asr: { num_threads: number; mode: string; model_variant: string };
  hotword: { enabled: boolean; file: string };
  inject: { method: string };
  general: { start_on_boot: boolean; show_tray: boolean; locale: string };
  overlay: { enabled: boolean };
  post: { punctuation_enabled: boolean };
};

type SetupPhase =
  | "waiting_permissions"
  | "installing_models"
  | "loading_engine"
  | "ready"
  | "error";

type PermissionsSnapshot = {
  all_granted: boolean;
  voice_ready: boolean;
  setup_phase: SetupPhase;
  setup_message: string | null;
  permissions: Array<{
    id: string;
    label: string;
    description: string;
    granted: boolean;
    action_label: string;
  }>;
  reinstall_tip: string | null;
};

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing element #${id}`);
  return el as T;
}

function voiceStatusText(snap: PermissionsSnapshot): string {
  if (snap.voice_ready) {
    return t("settings.voice.ready");
  }
  if (!snap.all_granted) {
    return t("settings.voice.not_ready");
  }
  switch (snap.setup_phase) {
    case "installing_models":
      return snap.setup_message ?? t("settings.voice.installing");
    case "loading_engine":
      return snap.setup_message ?? t("settings.voice.loading");
    case "error":
      return snap.setup_message ?? t("settings.voice.error");
    default:
      return t("settings.voice.starting");
  }
}

function renderPermissions(snap: PermissionsSnapshot) {
  const voiceStatus = byId<HTMLParagraphElement>("voice-ready-status");
  voiceStatus.textContent = voiceStatusText(snap);
  voiceStatus.className = `voice-ready-status${
    snap.voice_ready ? " is-ready" : snap.setup_phase === "error" ? " is-error" : " is-blocked"
  }`;

  const list = byId<HTMLUListElement>("permissions-list");
  list.replaceChildren(
    ...snap.permissions.map((item) => {
      const li = document.createElement("li");
      li.className = `permission-row${item.granted ? " is-granted" : ""}`;

      const name = document.createElement("span");
      name.className = "permission-name";
      name.textContent = item.label;

      const desc = document.createElement("span");
      desc.className = "permission-desc";
      desc.textContent = item.description;

      const status = document.createElement("span");
      status.className = "permission-status";
      status.textContent = item.granted
        ? t("settings.perm.granted")
        : t("settings.perm.denied");

      li.append(name, desc, status);

      if (!item.granted) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "secondary-btn permission-action";
        btn.textContent = item.action_label || t("settings.perm.action.fallback");
        btn.addEventListener("click", async () => {
          btn.disabled = true;
          try {
            await invoke("open_permission_settings", { permissionId: item.id });
            byId<HTMLParagraphElement>("permissions-tip").textContent =
              item.id === "accessibility"
                ? t("settings.perm.tip.accessibility_opened")
                : t("settings.perm.tip.settings_opened");
            byId<HTMLParagraphElement>("permissions-tip").classList.remove("hidden");
          } catch (err) {
            console.error(err);
            byId<HTMLParagraphElement>("permissions-tip").textContent =
              t("settings.perm.tip.open_failed");
            byId<HTMLParagraphElement>("permissions-tip").classList.remove("hidden");
          } finally {
            btn.disabled = false;
          }
        });
        li.append(btn);
      }

      return li;
    }),
  );

  const tip = byId<HTMLParagraphElement>("permissions-tip");
  if (snap.reinstall_tip) {
    tip.textContent = snap.reinstall_tip;
    tip.classList.remove("hidden");
  } else {
    tip.textContent = "";
    tip.classList.add("hidden");
  }
}

async function loadPermissions() {
  const snap = await invoke<PermissionsSnapshot>("get_permissions_status");
  renderPermissions(snap);
}

function fillForm(cfg: AppConfig) {
  byId<HTMLSelectElement>("locale").value = cfg.general.locale;
  byId<HTMLSelectElement>("trigger-key").value = cfg.hotkey.trigger_key;
  byId<HTMLSelectElement>("asr-mode").value = cfg.asr.mode;
  byId<HTMLInputElement>("silence-threshold").value = String(
    cfg.audio.silence_threshold_ms,
  );
  byId<HTMLInputElement>("min-speech-ms").value = String(cfg.audio.min_speech_ms);
  byId<HTMLSelectElement>("num-threads").value = String(cfg.asr.num_threads);
  byId<HTMLInputElement>("hotword-enabled").checked = cfg.hotword.enabled;
  byId<HTMLInputElement>("punctuation-enabled").checked = cfg.post.punctuation_enabled;
  byId<HTMLSelectElement>("inject-method").value = cfg.inject.method;
  byId<HTMLInputElement>("hotword-file").value = cfg.hotword.file;
  byId<HTMLInputElement>("overlay-enabled").checked = cfg.overlay.enabled;
  byId<HTMLInputElement>("show-tray").checked = cfg.general.show_tray;
  byId<HTMLInputElement>("start-on-boot").checked = cfg.general.start_on_boot;
}

function readForm(base: AppConfig): AppConfig {
  return {
    ...base,
    hotkey: {
      ...base.hotkey,
      trigger_key: byId<HTMLSelectElement>("trigger-key").value,
    },
    audio: {
      ...base.audio,
      silence_threshold_ms: Number(
        byId<HTMLInputElement>("silence-threshold").value,
      ),
      min_speech_ms: Number(byId<HTMLInputElement>("min-speech-ms").value),
    },
    asr: {
      ...base.asr,
      mode: byId<HTMLSelectElement>("asr-mode").value,
      num_threads: Number(byId<HTMLSelectElement>("num-threads").value),
    },
    inject: {
      method: byId<HTMLSelectElement>("inject-method").value,
    },
    hotword: {
      ...base.hotword,
      enabled: byId<HTMLInputElement>("hotword-enabled").checked,
      file: byId<HTMLInputElement>("hotword-file").value,
    },
    post: {
      ...base.post,
      punctuation_enabled: byId<HTMLInputElement>("punctuation-enabled").checked,
    },
    general: {
      ...base.general,
      locale: byId<HTMLSelectElement>("locale").value,
      show_tray: byId<HTMLInputElement>("show-tray").checked,
      start_on_boot: byId<HTMLInputElement>("start-on-boot").checked,
    },
    overlay: {
      enabled: byId<HTMLInputElement>("overlay-enabled").checked,
    },
  };
}

async function loadSettings() {
  const cfg = await invoke<AppConfig>("get_config");
  setLocale(cfg.general.locale);
  applyLocale();
  fillForm(cfg);

  byId<HTMLFormElement>("settings-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    const status = byId<HTMLSpanElement>("save-status");
    status.textContent = t("settings.status.saving");
    try {
      const updated = readForm(cfg);
      await invoke("save_config", { cfg: updated });
      Object.assign(cfg, updated);
      setLocale(updated.general.locale);
      applyLocale();
      status.textContent = t("settings.status.saved");
      await loadPermissions();
    } catch (err) {
      status.textContent = t("settings.status.save_failed");
      console.error(err);
    }
  });
}

window.addEventListener("DOMContentLoaded", () => {
  loadPermissions().catch(console.error);
  loadSettings().catch(console.error);

  listen("setup-updated", () => {
    loadPermissions().catch(console.error);
  }).catch(console.error);

  listen("locale-changed", (event) => {
    setLocale(String(event.payload));
    applyLocale();
    loadPermissions().catch(console.error);
  }).catch(console.error);

  window.setInterval(() => {
    if (document.visibilityState === "visible") {
      invoke<PermissionsSnapshot>("get_permissions_status")
        .then((snap) => {
          if (!snap.voice_ready) {
            renderPermissions(snap);
          }
        })
        .catch(console.error);
    }
  }, 2000);

  byId<HTMLButtonElement>("recheck-permissions").addEventListener("click", async () => {
    const btn = byId<HTMLButtonElement>("recheck-permissions");
    btn.disabled = true;
    btn.textContent = t("settings.status.rechecking");
    try {
      const snap = await invoke<PermissionsSnapshot>("recheck_permissions");
      renderPermissions(snap);
    } catch (err) {
      console.error(err);
    } finally {
      btn.disabled = false;
      btn.textContent = t("settings.btn.recheck_permissions");
    }
  });

  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") {
      invoke<PermissionsSnapshot>("recheck_permissions")
        .then(renderPermissions)
        .catch(console.error);
    }
  });
});
