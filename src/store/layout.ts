// Left column open/closed and width, persisted per machine (same behavior as Helm's).
import { create } from "zustand";

export const NAV_BOUNDS = { initial: 220, min: 180, max: 360 };

interface Layout {
  leftOpen: boolean;
  navWidth: number;
}

const KEY = "judgement-layout";

export const clampNav = (w: number) =>
  Number.isFinite(w) ? Math.max(NAV_BOUNDS.min, Math.min(NAV_BOUNDS.max, w)) : NAV_BOUNDS.initial;

function load(): Layout {
  try {
    const data = JSON.parse(localStorage.getItem(KEY) ?? "null") ?? {};
    return {
      leftOpen: typeof data.leftOpen === "boolean" ? data.leftOpen : true,
      navWidth: typeof data.navWidth === "number" ? clampNav(data.navWidth) : NAV_BOUNDS.initial,
    };
  } catch {
    return { leftOpen: true, navWidth: NAV_BOUNDS.initial };
  }
}

function persist(layout: Layout) {
  try {
    localStorage.setItem(KEY, JSON.stringify(layout));
  } catch {
    /* Layout remains usable when storage is unavailable. */
  }
}

interface LayoutStore extends Layout {
  toggleLeft: () => void;
  resizeNav: (width: number) => void;
}

export const useLayoutStore = create<LayoutStore>((set, get) => ({
  ...load(),
  toggleLeft: () => {
    const next = { leftOpen: !get().leftOpen, navWidth: get().navWidth };
    persist(next);
    set(next);
  },
  resizeNav: (width) => {
    const next = { leftOpen: get().leftOpen, navWidth: clampNav(width) };
    persist(next);
    set(next);
  },
}));
