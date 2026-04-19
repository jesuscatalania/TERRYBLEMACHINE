import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Modal } from "@/components/ui/Modal";
import { useWelcomeFlow } from "@/hooks/useWelcomeFlow";

const STEPS = [
  {
    title: "Welcome to TERRYBLEMACHINE",
    body: "Your local AI design tool. To start generating, add API keys for the providers you want to use in Settings.",
  },
  {
    title: "Flavor every output via meingeschmack/",
    body: "Optional: drop reference images, color palettes, and rules into the meingeschmack/ folder. Every generated output is flavored by what's there. See meingeschmack/README.md for details.",
  },
  {
    title: "Organize with projects",
    body: "Optional: create a project (Cmd+N) to organize generated assets per topic. You can ship without one, but projects keep history + exports per-topic.",
  },
] as const;

export function WelcomeModal() {
  const { open, dismiss } = useWelcomeFlow();
  const [step, setStep] = useState(0);
  if (!open) return null;
  const isLast = step === STEPS.length - 1;
  const current = STEPS[step];

  return (
    <Modal
      open={open}
      onClose={dismiss}
      title={current.title}
      maxWidth={480}
      footer={
        <>
          <Button variant="ghost" size="sm" onClick={dismiss}>
            Skip
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setStep((s) => Math.max(0, s - 1))}
            disabled={step === 0}
          >
            Back
          </Button>
          {isLast ? (
            <Button variant="primary" size="sm" onClick={dismiss}>
              Done
            </Button>
          ) : (
            <Button variant="primary" size="sm" onClick={() => setStep((s) => s + 1)}>
              Next
            </Button>
          )}
        </>
      }
    >
      <div className="flex flex-col gap-3 text-2xs text-neutral-dark-200 leading-relaxed">
        <p>{current.body}</p>
        <p className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
          Step {step + 1} of {STEPS.length}
        </p>
      </div>
    </Modal>
  );
}
