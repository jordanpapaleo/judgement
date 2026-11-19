// A fake Tauri backend for UI tests: each command name maps to a handler.
// Unhandled commands reject, so a test fails loudly if the UI calls something unexpected.

import { QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";
import type { ReactElement } from "react";
import type { Item } from "../bindings/Item";
import type { Learning } from "../bindings/Learning";
import type { Overview } from "../bindings/Overview";
import { makeQueryClient } from "../lib/queries";

type Handler = (args: Record<string, unknown>) => unknown;

export const backend = {
  handlers: {} as Record<string, Handler>,
  calls: [] as { cmd: string; args: Record<string, unknown> }[],
  reset() {
    this.handlers = {};
    this.calls = [];
  },
  on(cmd: string, handler: Handler) {
    this.handlers[cmd] = handler;
  },
  callsTo(cmd: string) {
    return this.calls.filter((c) => c.cmd === cmd).map((c) => c.args);
  },
};

export function renderWithClient(ui: ReactElement) {
  return render(<QueryClientProvider client={makeQueryClient()}>{ui}</QueryClientProvider>);
}

const NOW = "2026-11-18T12:00:00Z";

export function item(overrides: Partial<Item> = {}): Item {
  return {
    id: "01JITEM00000000000000000A1",
    title: "Parent tests own parent decisions",
    summary: "Test the decisions a component makes, not its children's output.",
    guidance: "Assert the props and slots a component passes to mocked children.",
    applies_when: "",
    exceptions: "",
    tags: ["testing"],
    phases: ["coding", "review"],
    repo_root: null,
    status: "active",
    omitted_at: null,
    source: "learn",
    source_harness: "claude-code",
    source_model: "claude-opus-5-5",
    evidence: ["The user cut five tests."],
    times_returned: 14,
    last_returned_at: NOW,
    reinforced_count: 3,
    created_at: NOW,
    updated_at: NOW,
    ...overrides,
  };
}

export function learning(overrides: Partial<Learning> = {}): Learning {
  return {
    id: "01JLEARN0000000000000000A1",
    source: "learn",
    kind: "new",
    target_id: null,
    payload: {
      title: "Check existing state before adding a store",
      summary: "Look for an existing store first.",
      guidance: "Search for an existing store before adding one.",
      applies_when: null,
      exceptions: null,
      tags: null,
      phases: null,
      repo_root: null,
    },
    evidence: "The user removed a duplicate store.",
    harness: "claude-code",
    model: "claude-opus-5-5",
    session_model: null,
    session_ref: null,
    signal_score: null,
    temperature: null,
    status: "pending",
    created_at: NOW,
    decided_at: null,
    ...overrides,
  };
}

export function overview(overrides: Partial<Overview["settings"]> = {}): Overview {
  return {
    settings: {
      participation: "active",
      auto_learn: "off",
      analyzer: { argv: ["claude", "-p"], model: "claude-opus-5-5" },
      updated_at: NOW,
      ...overrides,
    },
    pending: 1,
    index: { total: 1, visible: 1, index_bytes: 2150 },
  };
}
