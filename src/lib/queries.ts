// TanStack Query hooks over the Tauri commands. Every mutation waits for the
// committed result (no optimistic success) and then refetches everything; the
// backend also emits "store-changed" when another process (the CLI) commits.

import { QueryClient, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { api } from "./api";

export function makeQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, refetchOnWindowFocus: true, staleTime: 0 },
      mutations: { retry: false },
    },
  });
}

export const keys = {
  storeInfo: ["store-info"],
  overview: ["overview"],
  pending: ["pending"],
  items: ["items"],
  gauge: ["gauge"],
  backups: ["backups"],
  autostart: ["autostart"],
  cliLink: ["cli-link"],
} as const;

export const useStoreInfo = () => useQuery({ queryKey: keys.storeInfo, queryFn: api.storeInfo });
export const useOverview = () => useQuery({ queryKey: keys.overview, queryFn: api.overview });
export const usePending = () => useQuery({ queryKey: keys.pending, queryFn: api.listPending });
export const useItems = () => useQuery({ queryKey: keys.items, queryFn: api.listItems });
export const useGauge = () => useQuery({ queryKey: keys.gauge, queryFn: api.gauge });
export const useBackups = () => useQuery({ queryKey: keys.backups, queryFn: api.backupSummary });
export const useAutostart = () => useQuery({ queryKey: keys.autostart, queryFn: api.getAutostart });
export const useCliLink = () => useQuery({ queryKey: keys.cliLink, queryFn: api.cliLinkStatus });

/** A mutation that refetches all store data once the write has committed. */
export function useStoreMutation<A, R>(fn: (args: A) => Promise<R>) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSettled: () => client.invalidateQueries(),
  });
}

/** Refetch whenever the backend reports a committed change. */
export function useStoreChanges() {
  const client = useQueryClient();
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    listen("store-changed", () => client.invalidateQueries())
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [client]);
}
