// Workspace frame, matching Helm's AppShell: toolbar on top, a resizable left
// column (or its icon rail when closed), and the main panel.
import { type ReactNode, useEffect, useState } from "react";
import { useLayoutStore } from "../../store/layout";
import { useUIStore } from "../../store/ui";
import { SettingsModal } from "../settings/SettingsModal";
import { AppToolbar } from "./AppToolbar";
import { IconRail, LeftColumn } from "./LeftColumn";
import { PanelSeparator } from "./PanelSeparator";

const SKIPPED_TAGS = new Set(["INPUT", "TEXTAREA", "SELECT"]);

export function AppShell({ paused, children }: { paused: boolean | null; children: ReactNode }) {
  const { leftOpen, navWidth, resizeNav } = useLayoutStore();
  const { goBack, goForward, settingsTab, closeSettings } = useUIStore();
  const [preview, setPreview] = useState<number | null>(null);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (!e.metaKey || (e.key !== "ArrowLeft" && e.key !== "ArrowRight")) return;
      const target = e.target as HTMLElement;
      if (target.isContentEditable || SKIPPED_TAGS.has(target.tagName)) return;
      e.preventDefault();
      if (e.key === "ArrowLeft") goBack();
      else goForward();
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [goBack, goForward]);

  const width = preview ?? navWidth;
  return (
    <div className="flex h-screen min-w-0 flex-col overflow-hidden bg-base-100">
      <AppToolbar paused={paused} />
      <div className="flex min-h-0 min-w-0 flex-1 overflow-hidden">
        {leftOpen ? (
          <>
            <div style={{ width }} className="min-h-0 shrink-0">
              <LeftColumn />
            </div>
            <PanelSeparator width={width} onPreview={setPreview} onCommit={resizeNav} />
          </>
        ) : (
          <IconRail />
        )}
        <main className="min-w-0 flex-1 overflow-y-auto px-8 py-6">{children}</main>
      </div>
      {settingsTab && <SettingsModal tab={settingsTab} onClose={closeSettings} />}
    </div>
  );
}
