import { type FormEvent, useId, useState } from "react";
import { MODULES } from "@/components/shell/modules";
import { Button } from "@/components/ui/Button";
import { Dropdown } from "@/components/ui/Dropdown";
import { Input, Textarea } from "@/components/ui/Input";
import { LoadingButton } from "@/components/ui/LoadingButton";
import { Modal } from "@/components/ui/Modal";
import type { NewProjectInput } from "@/lib/projectCommands";
import { isProjectIpcError } from "@/lib/projectCommands";
import type { ModuleId } from "@/stores/appStore";

export interface NewProjectDialogProps {
  open: boolean;
  onClose: () => void;
  /** Persists the new project; reject to show an error message. */
  onCreate: (input: NewProjectInput) => Promise<void>;
  /** Pre-selected module in the dropdown. */
  defaultModule?: ModuleId;
}

export function NewProjectDialog({
  open,
  onClose,
  onCreate,
  defaultModule = "website",
}: NewProjectDialogProps) {
  const nameId = useId();
  const moduleId = useId();
  const descriptionId = useId();

  const [name, setName] = useState("");
  const [module, setModule] = useState<ModuleId>(defaultModule);
  const [description, setDescription] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reset = () => {
    setName("");
    setDescription("");
    setError(null);
    setBusy(false);
    setModule(defaultModule);
  };

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!name.trim() || busy) return;
    setBusy(true);
    setError(null);
    try {
      await onCreate({
        name: name.trim(),
        module,
        description: description.trim() || undefined,
      });
      reset();
      onClose();
    } catch (err) {
      if (isProjectIpcError(err)) {
        setError(err.detail || err.kind);
      } else if (err instanceof Error) {
        setError(err.message);
      } else {
        setError("Unknown error creating project");
      }
      setBusy(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={() => {
        if (!busy) {
          reset();
          onClose();
        }
      }}
      title="New project"
      maxWidth={520}
      footer={
        <>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              reset();
              onClose();
            }}
            disabled={busy}
          >
            Cancel
          </Button>
          <LoadingButton
            variant="primary"
            size="sm"
            type="submit"
            form="new-project-form"
            disabled={!name.trim()}
            loading={busy}
          >
            Create
          </LoadingButton>
        </>
      }
    >
      <form id="new-project-form" onSubmit={handleSubmit} className="flex flex-col gap-4">
        <Input
          id={nameId}
          label="Name"
          placeholder="Untitled"
          value={name}
          onValueChange={setName}
          disabled={busy}
          autoFocus
        />

        <div className="flex flex-col gap-1.5">
          <label
            htmlFor={moduleId}
            className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
          >
            Module
          </label>
          <Dropdown
            id={moduleId}
            value={module}
            onChange={(v) => setModule(v as ModuleId)}
            options={MODULES.map((m) => ({
              value: m.id,
              label: m.label,
              hint: m.tag,
            }))}
          />
        </div>

        <Textarea
          id={descriptionId}
          label="Description"
          placeholder="Optional: what is this project about?"
          value={description}
          onValueChange={setDescription}
          disabled={busy}
          maxHeight={180}
          rows={3}
        />

        {error ? (
          <div
            role="alert"
            className="rounded-xs border border-rose-500/40 bg-rose-500/10 px-3 py-2 font-mono text-2xs text-rose-300"
          >
            {error}
          </div>
        ) : null}
      </form>
    </Modal>
  );
}
