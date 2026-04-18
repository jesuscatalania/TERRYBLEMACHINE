import { Sparkles } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { SegmentList } from "@/components/video/SegmentList";
import { StoryboardEditor } from "@/components/video/StoryboardEditor";
import {
  generateStoryboard,
  type Storyboard,
  type StoryboardTemplate,
} from "@/lib/storyboardCommands";
import { useUiStore } from "@/stores/uiStore";
import { useVideoStore } from "@/stores/videoStore";

export function VideoPage() {
  const [prompt, setPrompt] = useState("");
  const [template, setTemplate] = useState<StoryboardTemplate>("commercial");
  const [busy, setBusy] = useState(false);
  const [storyboard, setStoryboard] = useState<Storyboard | null>(null);
  const [selectedSegmentId, setSelectedSegmentId] = useState<string | null>(null);
  const segments = useVideoStore((s) => s.segments);
  const removeSegment = useVideoStore((s) => s.removeSegment);
  const moveSegment = useVideoStore((s) => s.moveSegment);
  const notify = useUiStore((s) => s.notify);

  async function handleGenerate() {
    const trimmed = prompt.trim();
    if (!trimmed) return;
    setBusy(true);
    try {
      const sb = await generateStoryboard({
        prompt: trimmed,
        template,
        module: "video",
      });
      setStoryboard(sb);
      notify({
        kind: "success",
        message: `Storyboard ready · ${sb.shots.length} shots`,
      });
    } catch (err) {
      notify({
        kind: "error",
        message: "Storyboard generation failed",
        detail: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="grid h-full grid-rows-[auto_1fr]">
      {/* Brief row */}
      <div className="flex flex-col gap-3 border-neutral-dark-700 border-b p-6">
        <div className="flex items-center gap-2">
          <span className="font-mono text-2xs text-accent-500 uppercase tracking-label-wide">
            MOD—04 · VIDEO
          </span>
        </div>
        <div className="flex items-end gap-2">
          <div className="flex-1">
            <Input
              label="Describe the video"
              id="video-prompt"
              placeholder="30-second product spot for a coffee brand"
              value={prompt}
              onValueChange={setPrompt}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <label
              htmlFor="video-template"
              className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label"
            >
              Template
            </label>
            <select
              id="video-template"
              aria-label="Template"
              value={template}
              onChange={(e) => setTemplate(e.target.value as StoryboardTemplate)}
              className="rounded-xs border border-neutral-dark-600 bg-neutral-dark-800 px-3 py-2 text-sm text-neutral-dark-100"
            >
              <option value="commercial">Commercial</option>
              <option value="explainer">Explainer</option>
              <option value="social-media">Social Media</option>
              <option value="music-video">Music Video</option>
              <option value="custom">Custom</option>
            </select>
          </div>
          <Button variant="primary" onClick={handleGenerate} disabled={!prompt.trim() || busy}>
            <Sparkles className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
            {busy ? "Generating…" : "Generate storyboard"}
          </Button>
        </div>
      </div>

      {/* Split: toolbar / center (storyboard) / right (segments) */}
      <div className="grid min-h-0 grid-cols-[15rem_1fr_16rem]">
        <div className="flex flex-col gap-3 border-neutral-dark-700 border-r p-4">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Tools
          </span>
          {/* Tools placeholder — T12 wires export + render */}
        </div>

        <StoryboardEditor storyboard={storyboard} onChange={setStoryboard} />

        <div className="flex flex-col border-neutral-dark-700 border-l">
          <div className="flex items-center justify-between border-neutral-dark-700 border-b px-3 py-2">
            <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
              Segments · {segments.length}
            </span>
          </div>
          <div className="flex-1 overflow-y-auto">
            <SegmentList
              segments={segments}
              onDelete={removeSegment}
              onReorder={moveSegment}
              onSelect={setSelectedSegmentId}
              selectedId={selectedSegmentId}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
