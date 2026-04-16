import { Box, Film, Globe, Image, Type } from "lucide-react";
import { useState } from "react";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card, CardBody, CardFooter, CardHeader } from "@/components/ui/Card";
import { Dropdown } from "@/components/ui/Dropdown";
import { Input, NumberInput, Textarea } from "@/components/ui/Input";
import { Modal } from "@/components/ui/Modal";
import { Skeleton } from "@/components/ui/Skeleton";
import { Tabs } from "@/components/ui/Tabs";
import { Tooltip } from "@/components/ui/Tooltip";
import { useUiStore } from "@/stores/uiStore";

function Section({
  label,
  tag,
  children,
}: {
  label: string;
  tag: string;
  children: React.ReactNode;
}) {
  return (
    <section className="relative border border-neutral-dark-700 bg-neutral-dark-900/60">
      <div className="flex items-center justify-between border-neutral-dark-700 border-b px-4 py-2">
        <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
          {label}
        </span>
        <span className="font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
          {tag}
        </span>
      </div>
      <div className="p-6">{children}</div>
    </section>
  );
}

export function DesignSystemPage() {
  const [activeTab, setActiveTab] = useState("website");
  const [model, setModel] = useState("claude");
  const [modalOpen, setModalOpen] = useState(false);
  const notify = useUiStore((s) => s.notify);

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-5xl space-y-8 px-8 py-10">
        <header className="space-y-2">
          <div className="font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
            DS — 01
          </div>
          <h1 className="font-display text-3xl font-bold text-neutral-dark-50 tracking-tight">
            Design System
          </h1>
          <p className="max-w-xl text-neutral-dark-400">
            Reusable primitives for TERRYBLEMACHINE. Industrial / schematic, Inter body + IBM Plex
            Mono labels, Safety Orange accent.
          </p>
        </header>

        {/* Buttons */}
        <Section label="Buttons" tag="COMP—01">
          <div className="space-y-4">
            <div className="flex flex-wrap items-center gap-2">
              <Button variant="primary">Primary</Button>
              <Button variant="secondary">Secondary</Button>
              <Button variant="ghost">Ghost</Button>
              <Button variant="danger">Danger</Button>
              <Button disabled>Disabled</Button>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Button size="sm" variant="primary">
                Small
              </Button>
              <Button size="md" variant="primary">
                Medium
              </Button>
              <Button size="lg" variant="primary">
                Large
              </Button>
            </div>
          </div>
        </Section>

        {/* Inputs */}
        <Section label="Inputs" tag="COMP—02">
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <Input label="Name" id="ds-name" placeholder="Untitled" />
            <Input label="URL" id="ds-url" placeholder="https://…" error="Invalid URL" />
            <NumberInput label="Count" id="ds-count" placeholder="0" />
            <Textarea label="Prompt" id="ds-prompt" placeholder="Describe what to build…" />
          </div>
        </Section>

        {/* Badges */}
        <Section label="Badges" tag="COMP—03">
          <div className="flex flex-wrap items-center gap-2">
            <Badge tone="neutral">NEUTRAL</Badge>
            <Badge tone="success">READY</Badge>
            <Badge tone="warn">PENDING</Badge>
            <Badge tone="error">FAILED</Badge>
            <Badge tone="accent">NEW</Badge>
          </div>
        </Section>

        {/* Cards */}
        <Section label="Cards" tag="COMP—04">
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <Card>
              <CardHeader>Default card</CardHeader>
              <CardBody>Content area. Uses subtle border on the neutral-dark-800 surface.</CardBody>
              <CardFooter>
                <Button variant="ghost" size="sm">
                  Cancel
                </Button>
                <Button variant="primary" size="sm">
                  OK
                </Button>
              </CardFooter>
            </Card>
            <Card variant="schematic">
              <CardHeader>Schematic card</CardHeader>
              <CardBody>Industrial variant with orange corner brackets on a dark base.</CardBody>
            </Card>
          </div>
        </Section>

        {/* Tabs */}
        <Section label="Tabs" tag="COMP—05">
          <Tabs
            activeId={activeTab}
            onChange={setActiveTab}
            items={[
              { id: "website", label: "Website", icon: <Globe /> },
              { id: "graphic", label: "Graphic", icon: <Image /> },
              { id: "threed", label: "Pseudo-3D", icon: <Box /> },
              { id: "video", label: "Video", icon: <Film /> },
              { id: "type", label: "Type", icon: <Type /> },
            ]}
          />
          <p className="pt-4 text-neutral-dark-400 text-sm">
            Active: <span className="text-accent-500">{activeTab}</span>
          </p>
        </Section>

        {/* Dropdown */}
        <Section label="Dropdown (searchable)" tag="COMP—06">
          <Dropdown
            value={model}
            onChange={setModel}
            searchable
            options={[
              { value: "claude", label: "Claude Opus", hint: "Anthropic" },
              { value: "sonnet", label: "Claude Sonnet", hint: "Anthropic" },
              { value: "kling", label: "Kling 2.0", hint: "Video" },
              { value: "runway", label: "Runway Gen-3", hint: "Video" },
              { value: "fal", label: "fal.ai Flux", hint: "Image" },
              { value: "ideogram", label: "Ideogram v3", hint: "Typo" },
            ]}
          />
        </Section>

        {/* Tooltip */}
        <Section label="Tooltip" tag="COMP—07">
          <div className="flex gap-4">
            <Tooltip content="Runs the current prompt">
              <Button variant="primary">Generate</Button>
            </Tooltip>
            <Tooltip content="Opens settings" side="right">
              <Button variant="secondary">Hover right</Button>
            </Tooltip>
          </div>
        </Section>

        {/* Skeleton */}
        <Section label="Skeleton" tag="COMP—08">
          <div className="space-y-2">
            <Skeleton width="60%" height={18} />
            <Skeleton width="90%" height={14} />
            <Skeleton width="40%" height={14} />
          </div>
        </Section>

        {/* Modal */}
        <Section label="Modal" tag="COMP—09">
          <Button variant="primary" onClick={() => setModalOpen(true)}>
            Open modal
          </Button>
          <Modal
            open={modalOpen}
            onClose={() => setModalOpen(false)}
            title="Configure provider"
            footer={
              <>
                <Button variant="ghost" size="sm" onClick={() => setModalOpen(false)}>
                  Cancel
                </Button>
                <Button variant="primary" size="sm" onClick={() => setModalOpen(false)}>
                  Save
                </Button>
              </>
            }
          >
            <p className="text-neutral-dark-200">
              Example dialog with a footer action row. Dismiss via Escape, backdrop click, or the
              close button.
            </p>
          </Modal>
        </Section>

        {/* Toasts */}
        <Section label="Toasts" tag="COMP—10">
          <div className="flex flex-wrap gap-2">
            <Button onClick={() => notify({ kind: "success", message: "Saved." })}>Success</Button>
            <Button onClick={() => notify({ kind: "info", message: "Cache hit." })}>Info</Button>
            <Button
              onClick={() =>
                notify({ kind: "warning", message: "Budget at 80%.", detail: "Slow down." })
              }
            >
              Warning
            </Button>
            <Button
              onClick={() => notify({ kind: "error", message: "API failed.", detail: "HTTP 502" })}
            >
              Error
            </Button>
          </div>
        </Section>
      </div>
    </div>
  );
}
