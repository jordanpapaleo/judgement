// Small shared primitives on DaisyUI classes, so every theme from Helm applies.
import { Icon } from "@iconify/react";
import { type ReactNode, useState } from "react";

type ButtonVariant = "primary" | "secondary" | "quiet";

const BUTTON: Record<ButtonVariant, string> = {
  primary: "btn-primary",
  secondary: "btn-outline",
  quiet: "btn-ghost",
};

export function Button({
  variant = "secondary",
  className = "",
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant }) {
  return (
    <button type="button" className={`btn btn-sm ${BUTTON[variant]} ${className}`} {...props} />
  );
}

export function LinkButton({
  className = "",
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      type="button"
      className={`text-sm underline underline-offset-2 hover:text-error disabled:opacity-50 ${className}`}
      {...props}
    />
  );
}

/** Two-step destructive action: the first click asks, the second does it. */
export function ConfirmButton({
  label,
  confirmLabel,
  onConfirm,
  disabled,
  render = "link",
}: {
  label: string;
  confirmLabel: string;
  onConfirm: () => void;
  disabled?: boolean;
  render?: "link" | "button";
}) {
  const [asking, setAsking] = useState(false);
  if (asking) {
    return (
      <span className="inline-flex items-center gap-2">
        <button
          type="button"
          className="btn btn-sm btn-error"
          disabled={disabled}
          onClick={() => {
            setAsking(false);
            onConfirm();
          }}
        >
          {confirmLabel}
        </button>
        <LinkButton onClick={() => setAsking(false)}>Cancel</LinkButton>
      </span>
    );
  }
  if (render === "link") {
    return (
      <LinkButton disabled={disabled} onClick={() => setAsking(true)}>
        {label}
      </LinkButton>
    );
  }
  return (
    <Button disabled={disabled} onClick={() => setAsking(true)}>
      {label}
    </Button>
  );
}

export function Card({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <section className={`rounded-xl border border-base-300 bg-base-100 p-5 ${className}`}>
      {children}
    </section>
  );
}

export function PageHeader({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <header className="mb-5 flex flex-wrap items-end justify-between gap-4">
      <div>
        <h1 className="text-2xl font-bold">{title}</h1>
        {subtitle && <p className="mt-1 text-xs text-muted">{subtitle}</p>}
      </div>
      {children}
    </header>
  );
}

export function Badge({
  children,
  tone = "plain",
}: {
  children: ReactNode;
  tone?: "plain" | "primary" | "accent" | "warning";
}) {
  const tones = {
    plain: "badge-ghost",
    primary: "badge-primary badge-soft",
    accent: "badge-accent badge-soft",
    warning: "badge-warning badge-soft",
  };
  return <span className={`badge badge-sm font-semibold ${tones[tone]}`}>{children}</span>;
}

export function Notice({
  children,
  tone = "error",
}: {
  children: ReactNode;
  tone?: "error" | "info" | "paused";
}) {
  const tones = { error: "alert-error", info: "", paused: "alert-warning" };
  return (
    <div
      role={tone === "error" ? "alert" : "status"}
      className={`alert alert-soft py-2 text-sm ${tones[tone]}`}
    >
      {children}
    </div>
  );
}

export function Loading({ what }: { what: string }) {
  return <p className="py-10 text-center text-muted">Loading {what}…</p>;
}

export function Empty({ title, children }: { title: string; children?: ReactNode }) {
  return (
    <div className="rounded-xl border border-dashed border-base-300 px-6 py-12 text-center">
      <p className="font-semibold">{title}</p>
      {children && <p className="mt-1 text-sm text-muted">{children}</p>}
    </div>
  );
}

export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: ReactNode;
  children: ReactNode;
}) {
  return (
    // biome-ignore lint/a11y/noLabelWithoutControl: the control is passed as children
    <label className="block">
      <span className="block text-xs font-semibold">{label}</span>
      {hint && <span className="block text-xs text-muted">{hint}</span>}
      <span className="mt-1 block">{children}</span>
    </label>
  );
}

export const inputClass =
  "w-full rounded-md border border-base-300 bg-base-100 px-3 py-2 text-sm text-base-content placeholder:text-base-content/40 focus:border-primary focus:outline-none";

export function Select<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <label className="inline-flex items-center gap-2 text-xs text-muted">
      {label}
      <select
        className="select select-sm w-auto text-base-content"
        value={value}
        onChange={(e) => onChange(e.target.value as T)}
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </label>
  );
}

export function Meta({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div>
      <dt className="text-xs text-muted">{label}</dt>
      <dd className="text-sm font-semibold">{children}</dd>
    </div>
  );
}

/** Helm's header action: a quiet square icon button with a tooltip. */
export function IconButton({
  icon,
  label,
  busy,
  tone,
  className = "",
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  icon: string;
  label: string;
  busy?: boolean;
  tone?: "success" | "error";
}) {
  const tones = { success: "text-success opacity-100", error: "hover:text-error" };
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      className={`btn btn-ghost btn-sm btn-square opacity-60 hover:opacity-100 ${tone ? tones[tone] : ""} ${className}`}
      {...props}
    >
      {busy ? (
        <span className="loading loading-spinner loading-xs" aria-hidden="true" />
      ) : (
        <Icon icon={icon} className="h-5 w-5" aria-hidden="true" />
      )}
    </button>
  );
}

/** Thin vertical rule between groups of header actions. */
export function ActionDivider() {
  return <span aria-hidden="true" className="mx-1 h-5 w-px bg-base-300" />;
}

/** Destructive icon action: the first click asks, the second does it. */
export function IconConfirmButton({
  icon,
  label,
  confirmLabel,
  onConfirm,
  disabled,
}: {
  icon: string;
  label: string;
  confirmLabel: string;
  onConfirm: () => void;
  disabled?: boolean;
}) {
  const [asking, setAsking] = useState(false);
  if (!asking) {
    return (
      <IconButton
        icon={icon}
        label={label}
        tone="error"
        disabled={disabled}
        onClick={() => setAsking(true)}
      />
    );
  }
  return (
    <span className="inline-flex items-center gap-1">
      <button
        type="button"
        className="btn btn-error btn-sm"
        disabled={disabled}
        onClick={() => {
          setAsking(false);
          onConfirm();
        }}
      >
        {confirmLabel}
      </button>
      <IconButton icon="uil:times" label="Cancel" onClick={() => setAsking(false)} />
    </span>
  );
}
