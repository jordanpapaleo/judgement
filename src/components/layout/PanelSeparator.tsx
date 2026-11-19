// Drag or arrow-key resize for the left column. Adapted from Helm's PanelSeparator.
import { useEffect, useRef } from "react";
import { NAV_BOUNDS } from "../../store/layout";

export function PanelSeparator({
  width,
  onPreview,
  onCommit,
}: {
  width: number;
  onPreview: (width: number | null) => void;
  onCommit: (width: number) => void;
}) {
  const drag = useRef<{ x: number; width: number; value: number } | null>(null);
  const restore = useRef<(() => void) | null>(null);
  const previewCallback = useRef(onPreview);
  previewCallback.current = onPreview;
  useEffect(
    () => () => {
      restore.current?.();
      if (drag.current) previewCallback.current(null);
    },
    [],
  );
  const clamp = (value: number) => Math.max(NAV_BOUNDS.min, Math.min(NAV_BOUNDS.max, value));
  const end = () => {
    drag.current = null;
    restore.current?.();
    restore.current = null;
    onPreview(null);
  };
  return (
    // biome-ignore lint/a11y/useSemanticElements: focusable interactive window splitter, not a thematic break
    <div
      role="separator"
      tabIndex={0}
      aria-label="Resize navigation panel"
      aria-orientation="vertical"
      aria-valuemin={NAV_BOUNDS.min}
      aria-valuemax={NAV_BOUNDS.max}
      aria-valuenow={Math.round(width)}
      className="workspace-separator w-1 shrink-0 cursor-col-resize touch-none focus-visible:outline-none"
      onKeyDown={(event) => {
        if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
        event.preventDefault();
        const value = clamp(width + (event.key === "ArrowRight" ? 10 : -10));
        if (value !== width) onCommit(value);
      }}
      onPointerDown={(event) => {
        if (event.button !== 0 || drag.current) return;
        event.preventDefault();
        event.currentTarget.setPointerCapture(event.pointerId);
        drag.current = { x: event.clientX, width, value: width };
        const { cursor, userSelect } = document.body.style;
        document.body.style.cursor = "col-resize";
        document.body.style.userSelect = "none";
        restore.current = () => {
          document.body.style.cursor = cursor;
          document.body.style.userSelect = userSelect;
        };
      }}
      onPointerMove={(event) => {
        if (!drag.current) return;
        const value = clamp(drag.current.width + event.clientX - drag.current.x);
        drag.current.value = value;
        onPreview(value);
      }}
      onPointerUp={(event) => {
        if (!drag.current) return;
        const { value, width: initial } = drag.current;
        event.currentTarget.releasePointerCapture(event.pointerId);
        end();
        if (value !== initial) onCommit(value);
      }}
      onPointerCancel={end}
      onLostPointerCapture={() => {
        if (drag.current) end();
      }}
    />
  );
}
