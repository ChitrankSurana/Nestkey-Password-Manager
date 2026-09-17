import { Icon } from "./Icon";

// The single loading affordance for any operation with a perceptible wait (Argon2id
// derivations on login/reprompt/register run seconds long in dev/debug builds). Composed
// from the existing Icon choke point + Tailwind's animate-spin utility - no new dependency.
export function Spinner({ className = "" }: { className?: string }) {
  return (
    <span role="status" aria-label="Loading" className={className}>
      <Icon name="arrow-repeat" className="animate-spin" />
    </span>
  );
}
