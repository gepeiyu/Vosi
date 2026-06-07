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
  overlay: { enabled: boolean };
};

type PermissionsSnapshot = {
  all_granted: boolean;
  voice_ready: boolean;
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

function renderPermissions(snap: PermissionsSnapshot) {
  const voiceStatus = byId<HTMLParagraphElement>("voice-ready-status");
  voiceStatus.textContent = snap.voice_ready
    ? "语音功能：就绪"
    : snap.all_granted
      ? "语音功能：启动中…"
      : "语音功能：未就绪（请先开启下方全部权限）";
  voiceStatus.className = `voice-ready-status${snap.voice_ready ? " is-ready" : " is-blocked"}`;

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
      status.textContent = item.granted ? "已授权" : "未授权";

      li.append(name, desc, status);

      if (!item.granted) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "secondary-btn permission-action";
        btn.textContent = item.action_label || "去设置";
        btn.addEventListener("click", async () => {
          btn.disabled = true;
          try {
            await invoke("open_permission_settings", { permissionId: item.id });
            byId<HTMLParagraphElement>("permissions-tip").textContent =
              "已打开系统设置，请开启权限后返回并点击「重新检查权限」。";
            byId<HTMLParagraphElement>("permissions-tip").classList.remove("hidden");
          } catch (err) {
            console.error(err);
            byId<HTMLParagraphElement>("permissions-tip").textContent =
              "无法打开系统设置，请手动前往：系统设置 → 隐私与安全性";
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
  byId<HTMLSelectElement>("trigger-key").value = cfg.hotkey.trigger_key;
  byId<HTMLSelectElement>("asr-mode").value = cfg.asr.mode;
  byId<HTMLInputElement>("silence-threshold").value = String(
    cfg.audio.silence_threshold_ms,
  );
  byId<HTMLInputElement>("min-speech-ms").value = String(cfg.audio.min_speech_ms);
  byId<HTMLSelectElement>("num-threads").value = String(cfg.asr.num_threads);
  byId<HTMLInputElement>("hotword-enabled").checked = cfg.hotword.enabled;
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
    general: {
      ...base.general,
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
  loadPermissions().catch(console.error);
  loadSettings().catch(console.error);

  byId<HTMLButtonElement>("recheck-permissions").addEventListener("click", async () => {
    const btn = byId<HTMLButtonElement>("recheck-permissions");
    btn.disabled = true;
    btn.textContent = "检查中…";
    try {
      const snap = await invoke<PermissionsSnapshot>("recheck_permissions");
      renderPermissions(snap);
    } catch (err) {
      console.error(err);
    } finally {
      btn.disabled = false;
      btn.textContent = "重新检查权限";
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
