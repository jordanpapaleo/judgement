import type { StoreInfo } from "../bindings/StoreInfo";
import { Button, Card } from "../components/ui";
import { api } from "../lib/api";

/** Shown when the store won't open or migrate. The file is never touched. */
export function StoreError({ info }: { info: StoreInfo }) {
  return (
    <div className="flex h-full items-center justify-center p-8" data-tauri-drag-region>
      <Card className="max-w-xl">
        <p className="text-xs font-semibold text-error">▲ Store unavailable</p>
        <h1 className="mt-1 text-2xl font-bold">Judgement can't open its store</h1>
        <p className="mt-2 text-muted">
          The store didn't open or upgrade. It hasn't been touched, and agents continue without
          guidance.
        </p>
        <pre className="selectable mt-4 whitespace-pre-wrap rounded-lg bg-base-200 p-3 font-mono text-xs">
          {info.error}
        </pre>
        <p className="selectable mt-3 text-xs text-muted">
          Store: <code>{info.store_path}</code>
        </p>
        <p className="mt-3 text-xs text-muted">
          To restore a backup: quit Judgement, replace the store file with a copy from the backups
          folder, and relaunch.
        </p>
        <Button className="mt-4" onClick={() => api.revealBackups()}>
          Reveal Backups in Finder
        </Button>
      </Card>
    </div>
  );
}
