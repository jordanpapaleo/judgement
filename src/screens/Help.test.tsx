import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { renderWithClient } from "../test/backend";
import { AI_PREAMBLE, Help } from "./Help";

describe("Help", () => {
  it("renders the setup guide, including its tables", () => {
    renderWithClient(<Help />);
    expect(screen.getByRole("heading", { name: "Judgement Setup Guide" })).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /Setup checklist for an AI agent/ }),
    ).toBeInTheDocument();
    expect(screen.getAllByRole("table").length).toBeGreaterThan(3);
  });

  it("copies the guide with instructions for an AI", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    renderWithClient(<Help />);
    await userEvent.click(screen.getByRole("button", { name: "Copy for an AI" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledTimes(1));
    const text = writeText.mock.calls[0][0] as string;
    expect(text.startsWith(AI_PREAMBLE)).toBe(true);
    expect(text).toContain("# Judgement Setup Guide");
    expect(await screen.findByRole("button", { name: "Copied" })).toBeInTheDocument();
  });
});
