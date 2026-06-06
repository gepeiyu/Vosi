import { listen } from "@tauri-apps/api/event";

type OverlayPayload =
  | { phase: "hidden" }
  | { phase: "recording"; level: number }
  | { phase: "processing" };

const capsule = document.getElementById("capsule")!;
const recordingView = document.getElementById("recording-view")!;
const processingView = document.getElementById("processing-view")!;
const bars = document.getElementById("bars")!;

for (let i = 0; i < 5; i++) {
  const bar = document.createElement("div");
  bar.className = "bar";
  bars.appendChild(bar);
}

function setBars(level: number) {
  const barEls = bars.querySelectorAll<HTMLDivElement>(".bar");
  barEls.forEach((bar, i) => {
    const jitter = 0.85 + (i % 3) * 0.05;
    const h = Math.max(20, Math.min(100, level * 100 * jitter));
    bar.style.height = `${h}%`;
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
