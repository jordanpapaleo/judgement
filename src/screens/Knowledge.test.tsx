import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { backend, item, overview, renderWithClient } from "../test/backend";
import { Knowledge } from "./Knowledge";

const a = item({ id: "A", title: "Check existing capabilities first" });
const b = item({ id: "B", title: "Prefer existing abstractions" });

beforeEach(() => {
  backend.reset();
  backend.on("overview", () => overview());
  backend.on("list_items", () => [a, b]);
});

describe("Knowledge", () => {
  it("reports item and omit counts", async () => {
    renderWithClient(<Knowledge onEdit={vi.fn()} onCombine={vi.fn()} />);
    expect(await screen.findByText(/1 item · 0 omitted · index 2.1 KB/)).toBeInTheDocument();
  });

  it("omits an item with its Omit switch", async () => {
    backend.on("set_item_status", () => ({ ...a, status: "omitted" }));
    renderWithClient(<Knowledge onEdit={vi.fn()} onCombine={vi.fn()} />);
    await userEvent.click(await screen.findByLabelText(`Omit ${a.title}`));
    await waitFor(() =>
      expect(backend.callsTo("set_item_status")).toEqual([{ id: "A", status: "omitted" }]),
    );
  });

  it("combines only two or more selected items", async () => {
    const onCombine = vi.fn();
    renderWithClient(<Knowledge onEdit={vi.fn()} onCombine={onCombine} />);
    await userEvent.click(await screen.findByLabelText(`Select ${a.title}`));
    expect(screen.getByRole("button", { name: "Combine" })).toBeDisabled();
    await userEvent.click(screen.getByLabelText(`Select ${b.title}`));
    await userEvent.click(screen.getByRole("button", { name: "Combine" }));
    expect(onCombine).toHaveBeenCalledWith(["A", "B"]);
  });

  it("deletes selected items only after a second confirmation", async () => {
    backend.on("delete_items", () => 1);
    renderWithClient(<Knowledge onEdit={vi.fn()} onCombine={vi.fn()} />);
    await userEvent.click(await screen.findByLabelText(`Select ${a.title}`));
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(backend.callsTo("delete_items")).toHaveLength(0);
    await userEvent.click(screen.getByRole("button", { name: "Delete 1 permanently" }));
    await waitFor(() => expect(backend.callsTo("delete_items")).toEqual([{ ids: ["A"] }]));
  });

  it("shows the empty state", async () => {
    backend.on("list_items", () => []);
    renderWithClient(<Knowledge onEdit={vi.fn()} onCombine={vi.fn()} />);
    expect(await screen.findByText("No items yet")).toBeInTheDocument();
  });
});

describe("Knowledge selection", () => {
  beforeEach(() => {
    backend.reset();
    backend.on("overview", () => overview());
    backend.on("list_items", () => [a, b]);
  });

  it("selects every shown item from the header checkbox", async () => {
    renderWithClient(<Knowledge onEdit={vi.fn()} onCombine={vi.fn()} />);
    await userEvent.click(await screen.findByLabelText("Select all shown"));
    expect(screen.getByRole("toolbar", { name: "Selected items" })).toHaveTextContent("2 selected");
    expect(screen.getByRole("button", { name: "Combine" })).toBeEnabled();
  });

  it("uses a switch for Omit, separate from selection", async () => {
    renderWithClient(<Knowledge onEdit={vi.fn()} onCombine={vi.fn()} />);
    expect(await screen.findByLabelText(`Omit ${a.title}`)).toHaveClass("toggle");
    expect(screen.getByLabelText(`Select ${a.title}`)).toHaveClass("checkbox");
  });
});
