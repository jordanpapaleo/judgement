import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { EMPTY_FILTER } from "../../lib/knowledge";
import { useLayoutStore } from "../../store/layout";
import { useUIStore } from "../../store/ui";
import { backend, item, overview, renderWithClient } from "../../test/backend";
import { AppShell } from "./AppShell";

beforeEach(() => {
  backend.reset();
  backend.on("overview", () => overview());
  backend.on("list_items", () => [
    item({ tags: ["testing"] }),
    item({ id: "B", tags: ["react"], repo_root: "/w/ui/.git" }),
  ]);
  localStorage.clear();
  useLayoutStore.setState({ leftOpen: true, navWidth: 220 });
  useUIStore.setState({
    history: [{ name: "learnings" }],
    index: 0,
    screen: { name: "learnings" },
    filter: EMPTY_FILTER,
    settingsTab: null,
  });
});

describe("AppShell", () => {
  it("collapses the left column to an icon rail with Settings at the bottom", async () => {
    renderWithClient(<AppShell paused={false}>content</AppShell>);
    expect(await screen.findByText("Scopes")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Toggle left panel" }));

    const rail = screen.getByRole("navigation", { name: "Navigation" });
    expect(within(rail).queryByText("Scopes")).not.toBeInTheDocument();
    expect(within(rail).getByRole("button", { name: "Learnings" })).toBeInTheDocument();
    expect(within(rail).getByRole("button", { name: "Knowledge" })).toBeInTheDocument();
    await userEvent.click(within(rail).getByRole("button", { name: "Settings" }));
    expect(screen.getByRole("dialog", { name: "Settings" })).toBeInTheDocument();
    expect(JSON.parse(localStorage.getItem("judgement-layout") ?? "{}").leftOpen).toBe(false);
  });

  it("navigates with Back and Forward", async () => {
    renderWithClient(<AppShell paused={false}>content</AppShell>);
    const back = screen.getByRole("button", { name: "Back" });
    expect(back).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: /Knowledge/ }));
    expect(useUIStore.getState().screen).toEqual({ name: "knowledge" });
    await userEvent.click(back);
    expect(useUIStore.getState().screen).toEqual({ name: "learnings" });
    await userEvent.click(screen.getByRole("button", { name: "Forward" }));
    expect(useUIStore.getState().screen).toEqual({ name: "knowledge" });
  });

  it("filters Knowledge from the Tags and Scopes sections", async () => {
    renderWithClient(<AppShell paused={false}>content</AppShell>);
    await userEvent.click(await screen.findByRole("button", { name: /^#\s*react/ }));
    expect(useUIStore.getState().filter.tag).toBe("react");
    expect(useUIStore.getState().screen).toEqual({ name: "knowledge" });
    await userEvent.click(screen.getByRole("button", { name: /^ui/ }));
    expect(useUIStore.getState().filter.scope).toBe("/w/ui/.git");
  });

  it("pauses from the toolbar", async () => {
    backend.on("set_participation", () => overview().settings);
    renderWithClient(<AppShell paused={false}>content</AppShell>);
    await userEvent.click(screen.getByRole("button", { name: "Pause Judgement" }));
    expect(backend.callsTo("set_participation")).toEqual([{ participation: "paused" }]);
  });
});
