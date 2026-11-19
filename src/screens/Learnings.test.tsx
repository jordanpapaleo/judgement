import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { backend, item, learning, renderWithClient } from "../test/backend";
import { Learnings } from "./Learnings";

beforeEach(() => {
  backend.reset();
  backend.on("list_items", () => [item()]);
});

describe("Learnings", () => {
  it("shows the empty state when nothing is pending", async () => {
    backend.on("list_pending", () => []);
    renderWithClient(<Learnings onOpenItem={vi.fn()} />);
    expect(await screen.findByText("Nothing to review")).toBeInTheDocument();
  });

  it("confirms a new learning unedited", async () => {
    backend.on("list_pending", () => [learning()]);
    backend.on("confirm_learning", () => item());
    renderWithClient(<Learnings onOpenItem={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: "Confirm" }));
    await waitFor(() =>
      expect(backend.callsTo("confirm_learning")).toEqual([{ id: learning().id, edit: null }]),
    );
  });

  it("keeps the learning and shows the error when confirm fails", async () => {
    backend.on("list_pending", () => [learning()]);
    backend.on("confirm_learning", () => {
      throw "guidance looks like it contains an API key; describe it in prose instead (nothing was stored)";
    });
    renderWithClient(<Learnings onOpenItem={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: "Confirm" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("looks like it contains an API key");
    expect(
      screen.getByRole("heading", { name: learning().payload.title as string }),
    ).toBeInTheDocument();
  });

  it("edits a correction and confirms only the changed field", async () => {
    const target = item();
    const correction = learning({
      kind: "correct",
      source: "auto",
      temperature: "warm",
      signal_score: 3,
      target_id: target.id,
      payload: {
        ...learning().payload,
        title: null,
        summary: null,
        guidance: "Assert only the parent's decisions.",
      },
    });
    backend.on("list_items", () => [target]);
    backend.on("list_pending", () => [correction]);
    backend.on("confirm_learning", () => target);
    renderWithClient(<Learnings onOpenItem={vi.fn()} />);

    expect(await screen.findByText("3 of 4 · captured at Warm")).toBeInTheDocument();
    expect(screen.getByText("Current guidance")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Edit Then Confirm" }));
    expect(screen.queryByLabelText("Title")).not.toBeInTheDocument();
    const guidance = screen.getByLabelText("Guidance");
    await userEvent.clear(guidance);
    await userEvent.type(guidance, "Only the parent's decisions.");
    await userEvent.click(screen.getByRole("button", { name: "Confirm Edited" }));

    await waitFor(() => expect(backend.callsTo("confirm_learning")).toHaveLength(1));
    const { edit } = backend.callsTo("confirm_learning")[0] as { edit: Record<string, unknown> };
    expect(edit.guidance).toBe("Only the parent's decisions.");
    expect(edit.title).toBeNull();
  });

  it("disables confirm when the target item was deleted", async () => {
    backend.on("list_items", () => []);
    backend.on("list_pending", () => [
      learning({
        kind: "reinforce",
        target_id: "01JGONE0000000000000000000",
        payload: learning().payload,
      }),
    ]);
    renderWithClient(<Learnings onOpenItem={vi.fn()} />);
    expect(await screen.findByText(/targets was deleted/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Confirm" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Deny" })).toBeEnabled();
  });

  it("filters by model", async () => {
    backend.on("list_pending", () => [
      learning(),
      learning({
        id: "01JLEARN0000000000000000B2",
        model: "gpt-6.1-sol",
        payload: { ...learning().payload, title: "From GPT" },
      }),
    ]);
    renderWithClient(<Learnings onOpenItem={vi.fn()} />);
    await screen.findByRole("list", { name: "Pending learnings" });
    await userEvent.selectOptions(screen.getByLabelText("Model"), "gpt-6.1-sol");
    const list = screen.getByRole("list", { name: "Pending learnings" });
    expect(list).toHaveTextContent("From GPT");
    expect(list).not.toHaveTextContent("Check existing state");
  });

  it("denies and deletes with a second confirmation", async () => {
    backend.on("list_pending", () => [learning()]);
    backend.on("deny_learning", () => learning({ status: "denied" }));
    backend.on("delete_learning", () => null);
    renderWithClient(<Learnings onOpenItem={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: "Deny" }));
    await waitFor(() => expect(backend.callsTo("deny_learning")).toEqual([{ id: learning().id }]));

    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(backend.callsTo("delete_learning")).toHaveLength(0);
    await userEvent.click(screen.getByRole("button", { name: "Delete learning" }));
    await waitFor(() => expect(backend.callsTo("delete_learning")).toHaveLength(1));
  });
});
