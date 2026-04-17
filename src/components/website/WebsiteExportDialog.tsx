import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Dropdown } from "@/components/ui/Dropdown";
import { Modal } from "@/components/ui/Modal";

export type WebsiteExportFormat = "raw" | "react" | "next-js";
export type WebsiteDeployTarget = "vercel" | "netlify";

export interface WebsiteExportSettings {
  format: WebsiteExportFormat;
  /** When undefined, no deploy config is bundled. */
  deploy?: WebsiteDeployTarget;
}

export interface WebsiteExportDialogProps {
  open: boolean;
  onClose: () => void;
  onExport: (settings: WebsiteExportSettings) => void;
}

const FORMAT_OPTIONS = [
  { value: "raw", label: "Raw", hint: "Static files, no framework" },
  { value: "react", label: "React + Vite", hint: "Buildable Vite scaffold" },
  { value: "next-js", label: "Next.js", hint: "App Router scaffold" },
];

const DEPLOY_OPTIONS = [
  { value: "none", label: "None", hint: "Skip deploy config" },
  { value: "vercel", label: "Vercel", hint: "Adds vercel.json" },
  { value: "netlify", label: "Netlify", hint: "Adds netlify.toml" },
];

export function WebsiteExportDialog({ open, onClose, onExport }: WebsiteExportDialogProps) {
  const [format, setFormat] = useState<WebsiteExportFormat>("raw");
  const [deploy, setDeploy] = useState<"none" | WebsiteDeployTarget>("none");

  function handleExport() {
    onExport({
      format,
      deploy: deploy === "none" ? undefined : deploy,
    });
  }

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Export website"
      maxWidth={420}
      footer={
        <>
          <Button variant="ghost" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" size="sm" onClick={handleExport}>
            Export
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-1.5">
          <label
            htmlFor="website-export-format"
            className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
          >
            Format
          </label>
          <Dropdown
            id="website-export-format"
            value={format}
            onChange={(v) => setFormat(v as WebsiteExportFormat)}
            options={FORMAT_OPTIONS}
          />
        </div>

        <div className="flex flex-col gap-1.5">
          <label
            htmlFor="website-export-deploy"
            className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
          >
            Deploy target
          </label>
          <Dropdown
            id="website-export-deploy"
            value={deploy}
            onChange={(v) => setDeploy(v as "none" | WebsiteDeployTarget)}
            options={DEPLOY_OPTIONS}
          />
        </div>

        <p className="font-mono text-2xs text-neutral-dark-500 uppercase tracking-label">
          The archive writes to the project folder as a single `.zip`.
        </p>
      </div>
    </Modal>
  );
}
