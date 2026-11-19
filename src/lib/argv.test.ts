import { describe, expect, it } from "vitest";
import { joinArgs, splitArgs } from "./argv";

describe("splitArgs", () => {
  it("splits on whitespace and groups quoted words without expanding anything", () => {
    expect(splitArgs(`claude -p --allowedTools "Read Grep" '$HOME'`)).toEqual([
      "claude",
      "-p",
      "--allowedTools",
      "Read Grep",
      "$HOME",
    ]);
  });

  it("rejects an unclosed quote", () => {
    expect(() => splitArgs(`codex exec "oops`)).toThrow(/unclosed quote/);
  });

  it("round-trips through joinArgs", () => {
    const argv = ["codex", "exec", "--model", "gpt 6", "it's", ""];
    expect(splitArgs(joinArgs(argv))).toEqual(argv);
  });
});
