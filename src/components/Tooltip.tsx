import { useId, useState, ReactElement, cloneElement, MouseEvent, FocusEvent } from "react";

/**
 * Wraps a single focusable child (a <button> or <select>) and shows a small text tooltip on both
 * mouse hover AND keyboard focus. Deliberate replacement for `title=` across every icon/button —
 * `title` is hover-only in most webviews and doesn't reliably appear on keyboard Tab focus.
 */
export function Tooltip({ text, children }: { text: string; children: ReactElement }) {
  const [visible, setVisible] = useState(false);
  const id = useId();

  const child = cloneElement(children, {
    "aria-describedby": id,
    onMouseEnter: (e: MouseEvent) => {
      setVisible(true);
      (children.props as any).onMouseEnter?.(e);
    },
    onMouseLeave: (e: MouseEvent) => {
      setVisible(false);
      (children.props as any).onMouseLeave?.(e);
    },
    onFocus: (e: FocusEvent) => {
      setVisible(true);
      (children.props as any).onFocus?.(e);
    },
    onBlur: (e: FocusEvent) => {
      setVisible(false);
      (children.props as any).onBlur?.(e);
    },
  } as any);

  return (
    <span className="relative inline-block">
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
