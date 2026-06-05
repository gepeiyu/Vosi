import { invoke } from "@tauri-apps/api/core";

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
  general: { start_on_boot: boolean; show_tray: boolean };
};

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing element #${id}`);
  return el as T;
}

async function loadAccessibilityBanner() {
  const hint = await invoke<string | null>("get_accessibility_hint");
  if (!hint) return;
  const banner = byId<HTMLDivElement>("accessibility-banner");
  byId<HTMLParagraphElement>("accessibility-text").textContent = hint;
  banner.classList.remove("hidden");
  byId<HTMLButtonElement>("open-accessibility").addEventListener("click", () => {
    invoke("open_accessibility_settings").catch(console.error);
  });
}

function fillForm(cfg: AppConfig) {
  byId<HTMLSelectElement>("trigger-key").value = cfg.hotkey.trigger_key;
  byId<HTMLSelectElement>("asr-mode").value = cfg.asr.mode;
  byId<HTMLInputElement>("silence-threshold").value = String(
    cfg.audio.silence_threshold_ms,
  );
  byId<HTMLSelectElement>("inject-method").value = cfg.inject.method;
  byId<HTMLInputElement>("hotword-file").value = cfg.hotword.file;
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
    },
    asr: {
      ...base.asr,
      mode: byId<HTMLSelectElement>("asr-mode").value,
    },
    inject: {
      method: byId<HTMLSelectElement>("inject-method").value,
    },
    hotword: {
      ...base.hotword,
      file: byId<HTMLInputElement>("hotword-file").value,
    },
  };
}

async function loadSettings() {
  const cfg = await invoke<AppConfig>("get_config");
  fillForm(cfg);

  byId<HTMLFormElement>("settings-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    const status = byId<HTMLSpanElement>("save-status");
    status.textContent = "保存中…";
    try {
      const updated = readForm(cfg);
      await invoke("save_config", { cfg: updated });
      Object.assign(cfg, updated);
      status.textContent = "已保存";
    } catch (err) {
      status.textContent = "保存失败";
      console.error(err);
    }
  });
}

window.addEventListener("DOMContentLoaded", () => {
  loadAccessibilityBanner().catch(console.error);
  loadSettings().catch(console.error);
});
