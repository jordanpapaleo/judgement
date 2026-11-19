import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { backend, overview, renderWithClient } from "../../test/backend";
import { SettingsModal } from "./SettingsModal";

beforeEach(() => {
  backend.reset();
  backend.on("overview", () => overview());
  backend.on("gauge", () => ({
    since: "2026-10-19T00:00:00Z",
    by_temperature: [{ label: "warm", candidates: 6, confirmed: 3, denied: 1, sessions: 10 }],
    by_model: [{ label: "gpt-6.1-sol", candidates: 4, confirmed: 1, denied: 3, sessions: 4 }],
  }));
  backend.on("get_autostart", () => false);
  backend.on("backup_summary", () => ({
    dir: "/b",
    manual_count: 0,
    auto_count: 0,
    last_backup_at: null,
  }));
  backend.on("cli_link_status", () => ({
    link_path: "/Users/me/.local/bin/judgement",
    target: null,
    linked: false,
    conflict: null,
    in_applications: false,
  }));
  backend.on("store_info", () => ({
    store_path: "/s/judgement.sqlite",
    backups_dir: "/b",
    error: null,
  }));
});

describe("SettingsModal", () => {
  it("sets the temperature", async () => {
    backend.on("set_auto_learn", () => overview().settings);
    renderWithClient(<SettingsModal tab="agents" onClose={vi.fn()} />);
    await userEvent.click(await screen.findByRole("radio", { name: "Warm" }));
    await waitFor(() => expect(backend.callsTo("set_auto_learn")).toEqual([{ autoLearn: "warm" }]));
  });

  it("saves the analyzer as an argument array", async () => {
    backend.on("set_analyzer", () => overview().settings);
    renderWithClient(<SettingsModal tab="agents" onClose={vi.fn()} />);
    const command = await screen.findByLabelText(/Analyzer command/);
    await userEvent.clear(command);
    await userEvent.type(command, `codex exec --model "gpt 6.1" -`);
    await userEvent.click(screen.getByRole("button", { name: "Save Analyzer" }));
    await waitFor(() =>
      expect(backend.callsTo("set_analyzer")).toEqual([
        {
          analyzer: {
            argv: ["codex", "exec", "--model", "gpt 6.1", "-"],
            model: "claude-opus-5-5",
          },
        },
      ]),
    );
  });

  it("shows confirm rates from the gauge", async () => {
    renderWithClient(<SettingsModal tab="agents" onClose={vi.fn()} />);
    expect(await screen.findByText("75%")).toBeInTheDocument();
    expect(screen.getByText("25%")).toBeInTheDocument();
  });

  it("resumes when paused", async () => {
    backend.on("overview", () => overview({ participation: "paused" }));
    backend.on("set_participation", () => overview().settings);
    renderWithClient(<SettingsModal tab="agents" onClose={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: "Resume Judgement" }));
    await waitFor(() =>
      expect(backend.callsTo("set_participation")).toEqual([{ participation: "active" }]),
    );
  });

  it("switches themes and remembers the choice", async () => {
    renderWithClient(<SettingsModal tab="themes" onClose={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: /Dracula/ }));
    expect(document.documentElement.dataset.theme).toBe("dracula");
    expect(localStorage.getItem("judgement-theme")).toBe("dracula");
  });

  it("only offers the CLI link from the packaged app", async () => {
    renderWithClient(<SettingsModal tab="general" onClose={vi.fn()} />);
    expect(await screen.findByText(/only available in the packaged app/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Link judgement/ })).toBeDisabled();
  });

  it("closes on Escape", async () => {
    const onClose = vi.fn();
    renderWithClient(<SettingsModal tab="typography" onClose={onClose} />);
    await userEvent.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalled();
  });
});
