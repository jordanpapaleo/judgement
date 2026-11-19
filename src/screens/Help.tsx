// Help → Judgement Setup Guide. The same Markdown ships in the CLI
// (`judgement guide`), so a person and any AI read identical instructions.
import { useState } from "react";
import guide from "../../docs/setup-guide.md?raw";
import { Markdown } from "../components/Markdown";
import { IconButton, PageHeader } from "../components/ui";

/** Prepended when copying for an AI so it treats the guide as a task, safely. */
export const AI_PREAMBLE = `Please help me set up Judgement on this Mac by following the guide below,
starting with section 12 ("Setup checklist for an AI agent"). Verify each step
and report what you did. Ask me before changing any file in ~/.claude, ~/.agents,
~/.codex, or my shell profile, and show me what you will write.

---

`;

type Copied = "ai" | "guide" | null;

export function Help() {
  const [copied, setCopied] = useState<Copied>(null);
  const [failed, setFailed] = useState(false);

  const copy = (what: Exclude<Copied, null>) => {
    navigator.clipboard
      .writeText(what === "ai" ? AI_PREAMBLE + guide : guide)
      .then(() => {
        setFailed(false);
        setCopied(what);
        setTimeout(() => setCopied(null), 1500);
      })
      .catch(() => setFailed(true));
  };

  return (
    <div>
      <PageHeader
        title="Help"
        subtitle={
          <>
            Hand this guide to any AI agent to set Judgement up. In a terminal,{" "}
            <code>judgement guide</code> prints the same text.
          </>
        }
      >
        <div role="toolbar" aria-label="Guide actions" className="flex items-center gap-1">
          <button type="button" className="btn btn-primary btn-sm" onClick={() => copy("ai")}>
            {copied === "ai" ? "Copied" : "Copy for an AI"}
          </button>
          <IconButton
            icon={copied === "guide" ? "uil:check" : "uil:copy"}
            label="Copy guide"
            onClick={() => copy("guide")}
          />
        </div>
      </PageHeader>
      {failed && (
        <p role="alert" className="mb-4 text-sm text-error">
          Couldn't copy. Select the text below, or run <code>judgement guide</code> in a terminal.
        </p>
      )}
      <article className="rounded-xl border border-base-300 bg-base-100 p-8">
        <Markdown text={guide} variant="document" />
      </article>
    </div>
  );
}
