// Theme selection, persisted per machine. Same themes and storage shape as Helm.
import { create } from "zustand";
import { applyTheme, injectThemeStyles, THEMES, type Theme } from "../lib/themes";

const STORAGE_KEY = "judgement-theme";

function initialTheme(): Theme {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    return THEMES.find((t) => t.id === saved) ?? THEMES[0];
  } catch {
    return THEMES[0];
  }
}

interface ThemeStore {
  theme: Theme;
  setTheme: (id: string) => void;
}

export const useThemeStore = create<ThemeStore>((set) => {
  injectThemeStyles();
  const initial = initialTheme();
  applyTheme(initial);
  return {
    theme: initial,
    setTheme: (id) => {
      const theme = THEMES.find((t) => t.id === id) ?? THEMES[0];
      try {
        localStorage.setItem(STORAGE_KEY, id);
      } catch {
        /* The theme still applies for this session. */
      }
      applyTheme(theme);
      set({ theme });
    },
  };
});
