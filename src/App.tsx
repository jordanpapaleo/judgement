import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { AppShell } from "./components/layout/AppShell";
import { Loading, Notice } from "./components/ui";
import { errorMessage } from "./lib/api";
import { useOverview, useStoreChanges, useStoreInfo } from "./lib/queries";
import { Combine } from "./screens/Combine";
import { Help } from "./screens/Help";
import { ItemEditor } from "./screens/ItemEditor";
import { Knowledge } from "./screens/Knowledge";
import { Learnings } from "./screens/Learnings";
import { StoreError } from "./screens/StoreError";
import { useThemeStore } from "./store/theme";
import { useUIStore } from "./store/ui";

export default function App() {
  useThemeStore();
  const { screen, navigate, openSettings } = useUIStore();
  const info = useStoreInfo();
  const overview = useOverview();
  useStoreChanges();

  // The tray opens the window on a screen ("learnings") or on Settings.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    listen<string>("navigate", (e) => {
      if (e.payload === "learnings" || e.payload === "knowledge") navigate({ name: e.payload });
      else if (e.payload === "help") navigate({ name: "help" });
      else if (e.payload === "settings") openSettings();
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [navigate, openSettings]);

  if (info.isPending) return <Loading what="Judgement" />;
  if (info.data?.error) return <StoreError info={info.data} />;

  const participation = overview.data?.settings.participation;
  const toKnowledge = () => navigate({ name: "knowledge" });

  return (
    <AppShell paused={participation ? participation === "paused" : null}>
      {overview.isError && (
        <div className="mb-4">
          <Notice>{errorMessage(overview.error)}</Notice>
        </div>
      )}
      {participation === "paused" && (
        <div className="mb-4">
          <Notice tone="paused">
            Judgement is paused. Agents get no guidance and can't propose. You can still curate.
          </Notice>
        </div>
      )}
      {screen.name === "learnings" && (
        <Learnings onOpenItem={(id) => navigate({ name: "edit", id })} />
      )}
      {screen.name === "knowledge" && (
        <Knowledge
          onEdit={(id) => navigate({ name: "edit", id })}
          onCombine={(ids) => navigate({ name: "combine", ids })}
        />
      )}
      {screen.name === "edit" && <ItemEditor id={screen.id} onDone={toKnowledge} />}
      {screen.name === "combine" && <Combine ids={screen.ids} onDone={toKnowledge} />}
      {screen.name === "help" && <Help />}
    </AppShell>
  );
}
