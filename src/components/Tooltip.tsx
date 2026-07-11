import { useId, useState, ReactElement, cloneElement } from "react";

/**
 * Wraps a single focusable child (a <button> or <select>) and shows a small text tooltip on both
 * mouse hover AND keyboard focus. Deliberate replacement for `title=` across every icon/button —
 * `title` is hover-only in most webviews and doesn't reliably appear on keyboard Tab focus.
 *
 * Handlers live on the wrapper span, not cloned onto the child: this also makes hover work when
 * the child is disabled (a disabled form control's own mouse events are unreliable across
 * browsers, but hit-testing still resolves to this wrapper once pointer-events on the disabled
 * child are suppressed by the browser's UA stylesheet).
 */
export function Tooltip({
  text,
  children,
  className = "",
}: {
  text: string;
  children: ReactElement;
  className?: string;
}) {
  const [visible, setVisible] = useState(false);
  const id = useId();

  const child = cloneElement(children, { "aria-describedby": id } as any);

  return (
    <span
      className={`relative inline-block ${className}`}
      onMouseEnter={() => setVisible(true)}
      onMouseLeave={() => setVisible(false)}
      onFocus={() => setVisible(true)}
      onBlur={() => setVisible(false)}
    >
      {child}
      {visible && (
        <span
          id={id}
          role="tooltip"
          className="absolute z-10 -top-8 left-1/2 -translate-x-1/2 whitespace-nowrap rounded bg-slate-950 px-2 py-1 text-xs text-slate-100 shadow"
        >
          {text}
        </span>
      )}
    </span>
  );
}
