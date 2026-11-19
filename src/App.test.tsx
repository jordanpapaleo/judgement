import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import App from "./App";
import { backend, overview, renderWithClient } from "./test/backend";

beforeEach(() => {
  backend.reset();
  backend.on("list_pending", () => []);
  backend.on("list_items", () => []);
});

describe("App", () => {
  it("shows the store error instead of the app when the store can't open", async () => {
    backend.on("store_info", () => ({
      store_path: "/s/judgement.sqlite",
      backups_dir: "/b",
      error: "the store can't be read (file is not a database)",
    }));
    renderWithClient(<App />);
    expect(await screen.findByText("Judgement can't open its store")).toBeInTheDocument();
    expect(screen.getByText(/file is not a database/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Pause Judgement" })).not.toBeInTheDocument();
  });

  it("marks the paused state and offers Resume", async () => {
    backend.on("store_info", () => ({ store_path: "/s", backups_dir: "/b", error: null }));
    backend.on("overview", () => overview({ participation: "paused" }));
    renderWithClient(<App />);
    expect(await screen.findByText(/Judgement is paused/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Resume Judgement" })).toBeInTheDocument();
  });
});
