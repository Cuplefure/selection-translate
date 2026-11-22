import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./styles.css";

interface Timings {
  captureMs: number;
  translateMs: number;
}

interface Payload {
  ok: boolean;
  original: string;
  translated: string;
  sourceLang: string;
  cached: boolean;
  error: string | null;
  timings: Timings;
}

interface Settings {
  provider: string;
  endpoint: string;
  model: string;
  sourceLang: string;
  targetLang: string;
  autoCopy: boolean;
  hotkey: string;
  cacheEnabled: boolean;
}

const el = <T extends HTMLElement>(id: string): T => {
  const found = document.getElementById(id);
  if (!found) throw new Error(`missing element #${id}`);
  return found as T;
};

const win = getCurrentWindow();
const isSettings =
  win.label === "settings" || new URLSearchParams(location.search).get("view") === "settings";

function toast(target: HTMLElement, message: string): void {
  target.textContent = message;
  target.hidden = false;
  window.setTimeout(() => {
    target.hidden = true;
  }, 1800);
}

async function initCard(): Promise<void> {
  el<HTMLElement>("settings-view").hidden = true;
  el<HTMLElement>("card-view").hidden = false;
  const state = el<HTMLElement>("state");
  const sourceRow = el<HTMLElement>("source-row");
  const translated = el<HTMLElement>("translated");
  const badge = el<HTMLElement>("badge");
  const notice = el<HTMLElement>("toast");
  let current: Payload | null = null;

  const render = (payload: Payload): void => {
    current = payload;
    if (!payload.ok) {
      state.hidden = false;
      state.className = "state error";
      state.textContent = payload.error ?? "翻译失败";
      sourceRow.hidden = true;
      translated.textContent = "";
      badge.textContent = "";
      return;
    }
    state.hidden = true;
    sourceRow.hidden = false;
    el("source-lang").textContent = payload.sourceLang === "zh" ? "中文" : "自动检测";
    el("original").textContent = payload.original;
    translated.textContent = payload.translated;
    badge.textContent = payload.cached
      ? `缓存 · ${Math.round(payload.timings.translateMs)}ms`
      : `${Math.round(payload.timings.captureMs + payload.timings.translateMs)}ms`;
  };

  await listen<Payload>("translate-result", (event) => render(event.payload));
  await listen("translate-start", () => {
    state.hidden = false;
    state.className = "state";
    state.textContent = "翻译中…";
  });

  el<HTMLButtonElement>("translate").onclick = async () => {
    state.textContent = "翻译中…";
    try {
      await invoke("translate_now");
    } catch (error) {
      render({ ok: false, error: String(error), original: "", translated: "", sourceLang: "", cached: false, timings: { captureMs: 0, translateMs: 0 } });
    }
  };

  el<HTMLButtonElement>("copy").onclick = async () => {
    if (!current?.ok) return;
    try {
      await invoke("copy_text", { text: current.translated });
      toast(notice, "已复制英文");
    } catch (error) {
      toast(notice, String(error));
    }
  };

  el<HTMLButtonElement>("speak").onclick = () => {
    if (!current?.ok) return;
    const utterance = new SpeechSynthesisUtterance(current.translated);
    utterance.lang = "en-US";
    speechSynthesis.cancel();
    speechSynthesis.speak(utterance);
  };

  let pinned = false;
  el<HTMLButtonElement>("pin").onclick = async () => {
    pinned = !pinned;
    await win.setAlwaysOnTop(pinned);
    el("pin").textContent = pinned ? "取消固定" : "固定";
  };

  el<HTMLButtonElement>("close").onclick = async () => {
    await win.hide();
  };
}

async function initSettings(): Promise<void> {
  el<HTMLElement>("card-view").hidden = true;
  el<HTMLElement>("settings-view").hidden = false;
  const notice = el<HTMLElement>("settings-toast");

  try {
    const settings = await invoke<Settings>("get_settings");
    el<HTMLInputElement>("provider").value = settings.provider;
    el<HTMLInputElement>("endpoint").value = settings.endpoint;
    el<HTMLInputElement>("model").value = settings.model;
    el<HTMLInputElement>("hotkey").value = settings.hotkey;
    el<HTMLInputElement>("auto-copy").checked = settings.autoCopy;
    el<HTMLInputElement>("cache-enabled").checked = settings.cacheEnabled;
    if (await invoke<boolean>("has_api_key")) {
      el<HTMLInputElement>("api-key").placeholder = "已保存，留空表示不修改";
    }
  } catch (error) {
    toast(notice, String(error));
  }

  el<HTMLButtonElement>("save").onclick = async () => {
    const settings: Settings = {
      provider: el<HTMLInputElement>("provider").value,
      endpoint: el<HTMLInputElement>("endpoint").value,
      model: el<HTMLInputElement>("model").value,
      sourceLang: "auto",
      targetLang: "EN",
      autoCopy: el<HTMLInputElement>("auto-copy").checked,
      hotkey: el<HTMLInputElement>("hotkey").value,
      cacheEnabled: el<HTMLInputElement>("cache-enabled").checked,
    };
    const key = el<HTMLInputElement>("api-key").value;
    try {
      await invoke("save_settings", { settings });
      if (key.length > 0) await invoke("set_api_key", { key });
      el<HTMLInputElement>("api-key").value = "";
      toast(notice, "已保存");
    } catch (error) {
      toast(notice, String(error));
    }
  };

  el<HTMLButtonElement>("clear-cache").onclick = async () => {
    try {
      await invoke("clear_cache");
      toast(notice, "缓存已清空");
    } catch (error) {
      toast(notice, String(error));
    }
  };

  el<HTMLButtonElement>("close-settings").onclick = async () => {
    await win.hide();
  };
}

void (isSettings ? initSettings() : initCard());
