// Title-bar strip: left-panel toggle, Back/Forward, and the participation switch.
// Layout and controls match Helm's AppToolbar.
import { Icon } from "@iconify/react";
import { api } from "../../lib/api";
import { useStoreMutation } from "../../lib/queries";
import { useLayoutStore } from "../../store/layout";
import { useUIStore } from "../../store/ui";

function PanelIcon() {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      aria-hidden="true"
    >
      <rect x="3" y="4" width="18" height="16" rx="2" />
      <path d="M9 4v16" />
    </svg>
  );
}

export function AppToolbar({ paused }: { paused: boolean | null }) {
  const { goBack, goForward, index, history } = useUIStore();
  const { leftOpen, toggleLeft } = useLayoutStore();
  const toggle = useStoreMutation(() => api.setParticipation(paused ? "active" : "paused"));
  const macOS = /Mac/.test(navigator.platform);
  return (
    <div
      role="toolbar"
      aria-label="Workspace"
      data-tauri-drag-region
      className={`flex h-10 shrink-0 select-none items-center gap-1 border-b border-base-300 pr-3 ${macOS ? "pl-[84px]" : "pl-2"}`}
    >
      <button
        type="button"
        title="Toggle left panel"
        aria-label="Toggle left panel"
        aria-pressed={leftOpen}
        onClick={toggleLeft}
        className="workspace-toolbar-button mr-2"
      >
        <PanelIcon />
      </button>
      <button
        type="button"
        title="Back (⌘←)"
        aria-label="Back"
        disabled={index <= 0}
        onClick={goBack}
        className="workspace-toolbar-button"
      >
        <Icon icon="uil:arrow-left" width={24} height={24} aria-hidden="true" />
      </button>
      <button
        type="button"
        title="Forward (⌘→)"
        aria-label="Forward"
        disabled={index >= history.length - 1}
        onClick={goForward}
        className="workspace-toolbar-button"
      >
        <Icon icon="uil:arrow-right" width={24} height={24} aria-hidden="true" />
      </button>
      <div data-tauri-drag-region className="h-full flex-1" />
      {paused !== null && (
        <button
          type="button"
          title={paused ? "Resume Judgement" : "Pause Judgement"}
          aria-label={paused ? "Resume Judgement" : "Pause Judgement"}
          onClick={() => toggle.mutate(undefined)}
          disabled={toggle.isPending}
          className="flex h-7 items-center gap-1.5 rounded-full px-3 text-xs font-semibold hover:bg-base-200"
        >
          {paused ? (
            <Icon icon="uil:pause" className="h-4 w-4 text-warning" aria-hidden="true" />
          ) : (
            <span aria-hidden="true" className="h-2 w-2 rounded-full bg-success" />
          )}
          {paused ? "Paused" : "Active"}
        </button>
      )}
    </div>
  );
}
