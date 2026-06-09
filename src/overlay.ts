import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { applyLocale, setLocale } from "./i18n";

type OverlayPayload =
  | { phase: "hidden" }
  | { phase: "recording"; level: number }
  | { phase: "processing" };

const capsule = document.getElementById("capsule")!;
const recordingView = document.getElementById("recording-view")!;
const processingView = document.getElementById("processing-view")!;
const bars = document.getElementById("bars")!;

const BAR_COUNT = 5;
const BAR_MAX_PX = 22;
const BAR_MIN_PX = 6;

for (let i = 0; i < BAR_COUNT; i++) {
  const bar = document.createElement("div");
  bar.className = "bar";
  bars.appendChild(bar);
}

invoke<{ general: { locale: string } }>("get_config")
  .then((cfg) => {
    setLocale(cfg.general.locale);
    applyLocale();
  })
  .catch(console.error);

listen("locale-changed", (event) => {
  setLocale(String(event.payload));
  applyLocale();
}).catch(console.error);

/** Boost quiet mic RMS so bars move visibly during normal speech. */
function visualLevel(level: number): number {
  const boosted = Math.pow(Math.max(level, 0.0001), 0.42) * 1.75;
  return Math.min(1, boosted);
}

function setBars(level: number) {
  const visual = visualLevel(level);
  const barEls = bars.querySelectorAll<HTMLDivElement>(".bar");
  barEls.forEach((bar, i) => {
    const jitter = 0.78 + (i % BAR_COUNT) * 0.07;
    const h =
      BAR_MIN_PX + visual * jitter * (BAR_MAX_PX - BAR_MIN_PX);
    bar.style.height = `${Math.min(BAR_MAX_PX, h)}px`;
  });
}

function showRecording(level: number) {
  capsule.classList.remove("hidden");
  recordingView.classList.remove("hidden");
  processingView.classList.add("hidden");
  setBars(level);
}

function showProcessing() {
  capsule.classList.remove("hidden");
  recordingView.classList.add("hidden");
  processingView.classList.remove("hidden");
}

function hide() {
  capsule.classList.add("hidden");
  recordingView.classList.add("hidden");
  processingView.classList.add("hidden");
}

listen<OverlayPayload>("overlay-state", (event) => {
  const p = event.payload;
  if (p.phase === "hidden") hide();
  else if (p.phase === "recording") showRecording(p.level);
  else if (p.phase === "processing") showProcessing();
});
