export function RequiredMark() {
  return (
    <span className="ml-0.5 text-red-500" aria-hidden="true">
      *
    </span>
  );
}

export function RequiredLegend() {
  return (
    <p className="text-xs opacity-70">
      <span className="text-red-500">*</span> Required field
    </p>
  );
}
