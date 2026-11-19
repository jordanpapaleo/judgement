// Which screen is showing, with Back/Forward history like Helm's, plus the
// Knowledge filter (the sidebar's Scopes and Tags sections drive it).
import { create } from "zustand";
import { EMPTY_FILTER, type KnowledgeFilter } from "../lib/knowledge";

export type Section = "learnings" | "knowledge";

export type Screen =
  | { name: Section }
  | { name: "edit"; id: string }
  | { name: "combine"; ids: string[] }
  | { name: "help" };

export type SettingsTab = "agents" | "typography" | "themes" | "general";

const MAX_HISTORY = 100;

const same = (a: Screen, b: Screen) => JSON.stringify(a) === JSON.stringify(b);

interface UIStore {
  history: Screen[];
  index: number;
  screen: Screen;
  navigate: (screen: Screen) => void;
  goBack: () => void;
  goForward: () => void;

  filter: KnowledgeFilter;
  setFilter: (patch: Partial<KnowledgeFilter>) => void;

  settingsTab: SettingsTab | null;
  openSettings: (tab?: SettingsTab) => void;
  closeSettings: () => void;
}

const start: Screen = { name: "learnings" };

export const useUIStore = create<UIStore>((set, get) => ({
  history: [start],
  index: 0,
  screen: start,
  navigate: (screen) => {
    const { history, index } = get();
    if (same(history[index], screen)) return;
    const next = [...history.slice(0, index + 1), screen].slice(-MAX_HISTORY);
    set({ history: next, index: next.length - 1, screen });
  },
  goBack: () => {
    const { history, index } = get();
    if (index > 0) set({ index: index - 1, screen: history[index - 1] });
  },
  goForward: () => {
    const { history, index } = get();
    if (index < history.length - 1) set({ index: index + 1, screen: history[index + 1] });
  },

  filter: EMPTY_FILTER,
  setFilter: (patch) => set({ filter: { ...get().filter, ...patch } }),

  settingsTab: null,
  openSettings: (tab = "agents") => set({ settingsTab: tab }),
  closeSettings: () => set({ settingsTab: null }),
}));
