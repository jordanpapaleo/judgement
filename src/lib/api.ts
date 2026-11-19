// Typed wrappers around Tauri commands — the window's only path to the store.
// Command errors arrive as plain strings from Rust.

import { invoke } from "@tauri-apps/api/core";
import type { Analyzer } from "../bindings/Analyzer";
import type { AutoLearn } from "../bindings/AutoLearn";
import type { BackupSummary } from "../bindings/BackupSummary";
import type { CliLink } from "../bindings/CliLink";
import type { CombinePrefill } from "../bindings/CombinePrefill";
import type { Gauge } from "../bindings/Gauge";
import type { Item } from "../bindings/Item";
import type { ItemDraft } from "../bindings/ItemDraft";
import type { ItemStatus } from "../bindings/ItemStatus";
import type { Learning } from "../bindings/Learning";
import type { LearningPayload } from "../bindings/LearningPayload";
import type { Overview } from "../bindings/Overview";
import type { Participation } from "../bindings/Participation";
import type { Settings } from "../bindings/Settings";
import type { StoreInfo } from "../bindings/StoreInfo";

export const api = {
  storeInfo: () => invoke<StoreInfo>("store_info"),
  overview: () => invoke<Overview>("overview"),
  setParticipation: (participation: Participation) =>
    invoke<Settings>("set_participation", { participation }),
  setAutoLearn: (autoLearn: AutoLearn) => invoke<Settings>("set_auto_learn", { autoLearn }),
  setAnalyzer: (analyzer: Analyzer) => invoke<Settings>("set_analyzer", { analyzer }),

  listPending: () => invoke<Learning[]>("list_pending"),
  confirmLearning: (id: string, edit: LearningPayload | null) =>
    invoke<Item>("confirm_learning", { id, edit }),
  denyLearning: (id: string) => invoke<Learning>("deny_learning", { id }),
  deleteLearning: (id: string) => invoke<null>("delete_learning", { id }),

  listItems: () => invoke<Item[]>("list_items"),
  updateItem: (id: string, draft: ItemDraft) => invoke<Item>("update_item", { id, draft }),
  setItemStatus: (id: string, status: ItemStatus) =>
    invoke<Item>("set_item_status", { id, status }),
  deleteItems: (ids: string[]) => invoke<number>("delete_items", { ids }),
  combinePrefill: (ids: string[]) => invoke<CombinePrefill>("combine_prefill", { ids }),
  combineItems: (ids: string[], draft: ItemDraft) => invoke<Item>("combine_items", { ids, draft }),

  gauge: () => invoke<Gauge>("gauge"),
  backupSummary: () => invoke<BackupSummary>("backup_summary"),
  backupNow: () => invoke<BackupSummary>("backup_now"),
  revealBackups: () => invoke<null>("reveal_backups"),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<boolean>("set_autostart", { enabled }),
  cliLinkStatus: () => invoke<CliLink>("cli_link_status"),
  installCliLink: () => invoke<CliLink>("install_cli_link"),
};

export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "Something went wrong.";
}
