// Settings, in Helm's modal: tab bar, Typography / Themes / General like Helm,
// plus an Agents tab for participation and inception mode.
import { Icon } from "@iconify/react";
import { type ReactNode, useEffect, useState } from "react";
import type { AutoLearn } from "../../bindings/AutoLearn";
import type { GaugeRow } from "../../bindings/GaugeRow";
import type { Settings as StoreSettings } from "../../bindings/Settings";
import { api, errorMessage } from "../../lib/api";
import { joinArgs, splitArgs } from "../../lib/argv";
import { capitalize, relativeTime, TEMPERATURES } from "../../lib/format";
import {
  useAutostart,
  useBackups,
  useCliLink,
  useGauge,
  useOverview,
  useStoreInfo,
  useStoreMutation,
} from "../../lib/queries";
import { THEMES } from "../../lib/themes";
import { useSettingsStore } from "../../store/settings";
import { useThemeStore } from "../../store/theme";
import type { SettingsTab } from "../../store/ui";
import { inputClass, Notice } from "../ui";

const TABS: { id: SettingsTab; label: string; icon: string }[] = [
  { id: "agents", label: "Agents", icon: "uil:robot" },
  { id: "typography", label: "Typography", icon: "uil:text-size" },
  { id: "themes", label: "Themes", icon: "uil:palette" },
  { id: "general", label: "General", icon: "uil:sliders-v-alt" },
];

const TEMPERATURE_HELP: Record<AutoLearn, string> = {
  off: "Learn is your only capture tool. Nothing runs when a session ends.",
  cool: "Only explicit rules you state, like “never do X” or “we always Y”.",
  warm: "Explicit corrections and pushback, plus clear reinforcement like “yes, exactly”.",
  hot: "Any reaction, including implied preferences and one-offs. Expect noise.",
};

const HOOK_SNIPPET = `"hooks": {
  "SessionEnd": [
    { "hooks": [{ "type": "command", "command": "/Applications/Judgement.app/Contents/MacOS/judgement-cli hook session-end" }] }
  ]
}`;

export function SettingsModal({ tab, onClose }: { tab: SettingsTab; onClose: () => void }) {
  const [active, setActive] = useState<SettingsTab>(tab);
  useEffect(() => setActive(tab), [tab]);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: modal backdrop captures clicks outside the dialog
    // biome-ignore lint/a11y/useKeyWithClickEvents: Escape is handled on the document
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 pt-16"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Settings"
        className="flex flex-col rounded-2xl border border-base-300 bg-base-200 shadow-2xl"
        style={{ width: 760, maxHeight: "85vh" }}
      >
        <div className="relative flex shrink-0 items-center justify-center border-b border-base-300 px-6 py-4">
          <h2 className="text-base font-semibold">Settings</h2>
          <button
            type="button"
            onClick={onClose}
            className="btn btn-ghost btn-sm btn-circle absolute right-4"
            aria-label="Close settings"
          >
            <Icon icon="uil:times" className="h-5 w-5" aria-hidden="true" />
          </button>
        </div>
        <div role="tablist" className="tabs tabs-border shrink-0 px-4">
          {TABS.map((t) => (
            <button
              type="button"
              role="tab"
              key={t.id}
              aria-selected={active === t.id}
              onClick={() => setActive(t.id)}
              className={`tab gap-2 ${active === t.id ? "tab-active" : ""}`}
            >
              <Icon icon={t.icon} className="h-4 w-4" aria-hidden="true" />
              {t.label}
            </button>
          ))}
        </div>
        <div className="overflow-y-auto p-6">
          {active === "agents" && <AgentsTab />}
          {active === "typography" && <TypographyTab />}
          {active === "themes" && <ThemesTab />}
          {active === "general" && <GeneralTab />}
        </div>
      </div>
    </div>
  );
}

function Section({
  title,
  aside,
  children,
}: {
  title: string;
  aside?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="border-b border-base-300 pb-5 last:border-0 last:pb-0">
      <div className="mb-2 flex items-baseline justify-between">
        <h3 className="text-sm font-semibold">{title}</h3>
        {aside}
      </div>
      {children}
    </section>
  );
}

// ── Agents ──────────────────────────────────────────────────────────────────

function AgentsTab() {
  const overview = useOverview();
  const settings = overview.data?.settings;
  if (overview.isError) return <Notice>{errorMessage(overview.error)}</Notice>;
  if (!settings) return null;
  return (
    <div className="flex flex-col gap-5">
      <Participation settings={settings} />
      <Inception key={JSON.stringify(settings.analyzer)} settings={settings} />
      <TemperatureGauge />
    </div>
  );
}

function Participation({ settings }: { settings: StoreSettings }) {
  const paused = settings.participation === "paused";
  const toggle = useStoreMutation(() => api.setParticipation(paused ? "active" : "paused"));
  return (
    <Section title={`Participation · ${paused ? "Paused" : "Active"}`}>
      <div className="flex items-center justify-between gap-4">
        <p className="text-xs text-muted">
          Pause stops agents from recalling or proposing. Rules already written into your skills
          still apply.
        </p>
        <button
          type="button"
          className="btn btn-sm"
          disabled={toggle.isPending}
          onClick={() => toggle.mutate(undefined)}
        >
          {paused ? "Resume Judgement" : "Pause Judgement"}
        </button>
      </div>
      {toggle.error && (
        <div className="mt-3">
          <Notice>{errorMessage(toggle.error)}</Notice>
        </div>
      )}
    </Section>
  );
}

function Inception({ settings }: { settings: StoreSettings }) {
  const [command, setCommand] = useState(joinArgs(settings.analyzer.argv));
  const [model, setModel] = useState(settings.analyzer.model);
  const [parseError, setParseError] = useState<string | null>(null);
  const setTemperature = useStoreMutation((t: AutoLearn) => api.setAutoLearn(t));
  const saveAnalyzer = useStoreMutation((argv: string[]) => api.setAnalyzer({ argv, model }));
  const dirty = command !== joinArgs(settings.analyzer.argv) || model !== settings.analyzer.model;
  const error = setTemperature.error ?? saveAnalyzer.error;

  const save = () => {
    try {
      setParseError(null);
      saveAnalyzer.mutate(splitArgs(command));
    } catch (e) {
      setParseError(errorMessage(e));
    }
  };

  return (
    <Section title="Inception Mode">
      <p className="text-xs text-muted">
        Learns from what you say during Claude Code sessions. Everything it captures waits in
        Learnings for you to confirm or deny. Learn stays your default tool.
      </p>
      <fieldset className="join mt-3">
        <legend className="sr-only">Temperature</legend>
        {TEMPERATURES.map((t) => (
          <input
            key={t}
            type="radio"
            name="temperature"
            aria-label={capitalize(t)}
            className="join-item btn btn-sm w-20"
            checked={settings.auto_learn === t}
            disabled={setTemperature.isPending}
            onChange={() => setTemperature.mutate(t)}
          />
        ))}
      </fieldset>
      <p className="mt-2 text-sm">
        <span className="font-semibold">{capitalize(settings.auto_learn)}:</span>{" "}
        {TEMPERATURE_HELP[settings.auto_learn]}
      </p>

      <div className="mt-4 grid grid-cols-2 gap-4">
        <label className="block">
          <span className="block text-xs font-semibold">Analyzer command</span>
          <input
            className={`${inputClass} mt-1 font-mono`}
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            spellCheck={false}
          />
          <span className="mt-1 block text-xs text-muted">
            Any model's headless mode, including an OpenAI harness. Run without a shell.
          </span>
        </label>
        <label className="block">
          <span className="block text-xs font-semibold">Analyzer model label</span>
          <input
            className={`${inputClass} mt-1 font-mono`}
            value={model}
            onChange={(e) => setModel(e.target.value)}
            spellCheck={false}
          />
          <span className="mt-1 block text-xs text-muted">
            Recorded on every learning it writes.
          </span>
        </label>
      </div>
      <div className="mt-3 flex items-center gap-3">
        <button
          type="button"
          className="btn btn-sm"
          disabled={!dirty || saveAnalyzer.isPending}
          onClick={save}
        >
          {saveAnalyzer.isPending ? "Saving…" : "Save Analyzer"}
        </button>
        {(parseError || error) && <Notice>{parseError ?? errorMessage(error)}</Notice>}
      </div>
      <details className="mt-4 text-xs text-muted">
        <summary className="cursor-pointer">Connect Claude Code</summary>
        <p className="mt-2">
          Add this to <code>~/.claude/settings.json</code> yourself. Judgement never edits it. The
          hook exits immediately while paused or Off.
        </p>
        <pre className="selectable mt-2 overflow-x-auto rounded-lg bg-base-100 p-3 font-mono text-xs text-base-content">
          {HOOK_SNIPPET}
        </pre>
      </details>
    </Section>
  );
}

function rate(row: GaugeRow): number | null {
  const decided = row.confirmed + row.denied;
  return decided === 0 ? null : row.confirmed / decided;
}

function Bar({ row, tone, perSession }: { row: GaugeRow; tone: string; perSession: boolean }) {
  const r = rate(row);
  return (
    <li className="grid grid-cols-[110px_1fr_auto] items-center gap-3 text-sm">
      <span className="truncate" title={row.label}>
        {capitalize(row.label)}
      </span>
      <progress className={`progress ${tone} h-2`} value={Math.round((r ?? 0) * 100)} max={100} />
      <span className="w-32 text-right text-xs">
        <strong>{r === null ? "—" : `${Math.round(r * 100)}%`}</strong>
        {perSession &&
          row.sessions > 0 &&
          ` ${(row.candidates / row.sessions).toFixed(1)} / session`}
        {!perSession && ` of ${row.confirmed + row.denied}`}
      </span>
    </li>
  );
}

function TemperatureGauge() {
  const gauge = useGauge();
  const g = gauge.data;
  return (
    <Section
      title="Temperature Gauge"
      aside={<span className="text-xs text-muted">Last 30 days</span>}
    >
      {gauge.isError && <Notice>{errorMessage(gauge.error)}</Notice>}
      {g && (
        <div className="grid grid-cols-2 gap-6">
          <div>
            <h4 className="mb-2 text-xs font-semibold">Confirm rate by temperature</h4>
            {g.by_temperature.length === 0 ? (
              <p className="text-xs text-muted">No inception-mode learnings yet.</p>
            ) : (
              <ul className="space-y-2">
                {g.by_temperature.map((row) => (
                  <Bar key={row.label} row={row} tone="progress-accent" perSession />
                ))}
              </ul>
            )}
          </div>
          <div>
            <h4 className="mb-2 text-xs font-semibold">Confirm rate by model</h4>
            {g.by_model.length === 0 ? (
              <p className="text-xs text-muted">No Learn or inception-mode learnings yet.</p>
            ) : (
              <ul className="space-y-2">
                {g.by_model.map((row) => (
                  <Bar key={row.label} row={row} tone="progress-primary" perSession={false} />
                ))}
              </ul>
            )}
            <p className="mt-2 text-xs text-muted">
              Covers learnings from Learn and inception mode.
            </p>
          </div>
        </div>
      )}
      <p className="mt-4 rounded-lg bg-base-100 px-3 py-2 text-xs">
        Mostly denied? Turn it cooler. Quiet and mostly confirmed? Turn it warmer.
      </p>
    </Section>
  );
}

// ── Typography (Helm's sliders, applied to guidance text) ───────────────────

function SliderRow({
  label,
  value,
  min,
  max,
  step,
  unit,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  unit: string;
  onChange: (value: number) => void;
}) {
  return (
    <label className="flex items-center gap-4">
      <span className="w-36 shrink-0 text-right text-sm opacity-50">{label}</span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="range range-accent range-sm flex-1"
      />
      <span className="w-16 shrink-0 text-right text-sm tabular-nums">
        {value}
        {unit}
      </span>
    </label>
  );
}

function TypographyTab() {
  const { display, update, reset } = useSettingsStore();
  return (
    <div className="flex flex-col gap-5">
      <SliderRow
        label="Font Size"
        value={display.fontSize}
        min={12}
        max={20}
        step={1}
        unit="pt"
        onChange={(fontSize) => update({ fontSize })}
      />
      <SliderRow
        label="Line Height"
        value={display.lineHeight}
        min={1.2}
        max={2.2}
        step={0.1}
        unit="em"
        onChange={(lineHeight) => update({ lineHeight })}
      />
      <p className="prose-guidance rounded-lg bg-base-100 p-4">
        Assert only the decisions the parent makes. Don't re-test what a child, helper or hook
        already covers.
      </p>
      <div className="border-t border-base-300 pt-4">
        <button type="button" onClick={reset} className="btn btn-ghost btn-sm">
          Restore Defaults
        </button>
      </div>
    </div>
  );
}

// ── Themes (identical to Helm's picker) ────────────────────────────────────

function ThemesTab() {
  const { theme, setTheme } = useThemeStore();
  return (
    <div className="grid grid-cols-3 gap-3">
      {[...THEMES]
        .sort((a, b) =>
          a.colorScheme !== b.colorScheme
            ? a.colorScheme === "light"
              ? -1
              : 1
            : a.name.localeCompare(b.name),
        )
        .map((t) => {
          const isActive = theme.id === t.id;
          return (
            <button
              type="button"
              key={t.id}
              data-theme={t.id}
              aria-pressed={isActive}
              onClick={() => setTheme(t.id)}
              className="rounded-xl border-2 p-4 text-left transition-all"
              style={{
                backgroundColor: "var(--color-base-100)",
                color: "var(--color-base-content)",
                borderColor: isActive ? "var(--color-primary)" : "var(--color-border)",
                boxShadow: isActive ? "0 0 0 1px var(--color-primary)" : "none",
              }}
            >
              <p className="mb-2 text-sm font-bold">{t.name}</p>
              <p className="text-xs leading-relaxed" style={{ color: "var(--color-text-muted)" }}>
                Assert only{" "}
                <strong style={{ color: "var(--color-base-content)" }}>
                  the parent's decisions
                </strong>
                , not its <span style={{ color: "var(--color-accent)" }}>children's</span> output.
              </p>
            </button>
          );
        })}
    </div>
  );
}

// ── General ────────────────────────────────────────────────────────────────

function GeneralTab() {
  const autostart = useAutostart();
  const backups = useBackups();
  const cliLink = useCliLink();
  const info = useStoreInfo();
  const setAutostart = useStoreMutation((enabled: boolean) => api.setAutostart(enabled));
  const backupNow = useStoreMutation(() => api.backupNow());
  const link = useStoreMutation(() => api.installCliLink());
  const error = setAutostart.error ?? backupNow.error ?? link.error ?? autostart.error;
  const b = backups.data;
  const l = cliLink.data;

  return (
    <div className="flex flex-col gap-5">
      <Section title="Startup">
        <label className="flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2.5 hover:bg-base-300/50">
          <input
            type="checkbox"
            className="rounded"
            checked={autostart.data ?? false}
            disabled={autostart.isPending || setAutostart.isPending}
            onChange={(e) => setAutostart.mutate(e.target.checked)}
          />
          <span className="text-sm">Launch at login</span>
        </label>
      </Section>

      <Section title="Backups">
        <div className="flex flex-wrap items-center gap-3">
          <button
            type="button"
            className="btn btn-sm"
            disabled={backupNow.isPending}
            onClick={() => backupNow.mutate(undefined)}
          >
            {backupNow.isPending ? "Backing Up…" : "Back Up Now"}
          </button>
          <button
            type="button"
            className="btn btn-ghost btn-sm"
            onClick={() => api.revealBackups()}
          >
            Reveal in Finder
          </button>
        </div>
        <p className="mt-2 text-xs text-muted">
          {b?.last_backup_at ? `Last backup ${relativeTime(b.last_backup_at)}` : "No backups yet"}
          {b && ` · ${b.manual_count} kept`} · restore by quitting and replacing the store file
        </p>
      </Section>

      <Section title="Command-line tool">
        {l?.linked ? (
          <p className="text-xs text-muted">
            <code>{l.link_path}</code> → the bundled CLI.
          </p>
        ) : (
          <div className="flex flex-wrap items-center gap-3">
            <button
              type="button"
              className="btn btn-sm"
              disabled={!l?.target || !!l?.conflict || link.isPending}
              onClick={() => link.mutate(undefined)}
            >
              Link judgement into ~/.local/bin
            </button>
            <span className="text-xs text-muted">
              {l?.conflict
                ? `Can't link: ${l.conflict}.`
                : l?.target
                  ? "Creates one symlink. Your shell profile isn't touched."
                  : "The bundled CLI is only available in the packaged app."}
            </span>
          </div>
        )}
        {l?.target && !l.in_applications && (
          <p className="mt-1 text-xs text-warning">
            Judgement isn't in /Applications; the link breaks if the app moves.
          </p>
        )}
        {info.data && (
          <p className="selectable mt-3 text-xs text-muted">
            Store: <code>{info.data.store_path}</code>
          </p>
        )}
      </Section>
      {error && <Notice>{errorMessage(error)}</Notice>}
    </div>
  );
}
