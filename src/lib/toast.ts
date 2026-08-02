// Module-level subscriber list, not a React context - this app has two separate roots (App.tsx,
// ViewerApp.tsx) that would otherwise each need their own provider threaded through every screen.
type Listener = (message: string) => void;

const listeners = new Set<Listener>();

export function toast(message: string): void {
  for (const listen of listeners) listen(message);
}

export function subscribeToast(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
