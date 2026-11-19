import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";

// Every test talks to the fake backend in ./backend instead of Tauri.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
    const { backend } = await import("./backend");
    backend.calls.push({ cmd, args });
    const handler = backend.handlers[cmd];
    if (!handler) throw `unhandled command ${cmd}`;
    return handler(args);
  },
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: async () => () => {},
}));
