// Display preferences (per machine, not in the store): typography for guidance text.
import { create } from "zustand";

export interface DisplaySettings {
  fontSize: number;
  lineHeight: number;
}

export const DEFAULT_DISPLAY: DisplaySettings = { fontSize: 14, lineHeight: 1.6 };

const STORAGE_KEY = "judgement-display";

function load(): DisplaySettings {
  try {
    return { ...DEFAULT_DISPLAY, ...JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}") };
  } catch {
    return DEFAULT_DISPLAY;
  }
}

function apply(s: DisplaySettings) {
  const root = document.documentElement;
  root.style.setProperty("--content-font-size", `${s.fontSize}px`);
  root.style.setProperty("--content-line-height", String(s.lineHeight));
}

interface SettingsStore {
  display: DisplaySettings;
  update: (patch: Partial<DisplaySettings>) => void;
  reset: () => void;
}

function save(display: DisplaySettings) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(display));
  } catch {
    /* Preferences still apply for this session. */
  }
  apply(display);
}

export const useSettingsStore = create<SettingsStore>((set, get) => {
  const initial = load();
  apply(initial);
  return {
    display: initial,
    update: (patch) => {
      const display = { ...get().display, ...patch };
      save(display);
      set({ display });
    },
    reset: () => {
      save(DEFAULT_DISPLAY);
      set({ display: DEFAULT_DISPLAY });
    },
  };
});
