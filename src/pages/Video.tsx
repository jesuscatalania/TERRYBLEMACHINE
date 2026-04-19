import { convertFileSrc } from "@tauri-apps/api/core";
import { Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { LoadingButton } from "@/components/ui/LoadingButton";
import { RenderExportDialog, type RenderSettings } from "@/components/video/RenderExportDialog";
import { SegmentList } from "@/components/video/SegmentList";
import { StoryboardEditor } from "@/components/video/StoryboardEditor";
import { type AssemblyResult, assembleVideo } from "@/lib/assemblyCommands";
import { renderRemotion } from "@/lib/remotionCommands";
import {
  generateStoryboard,
  type Storyboard,
  type StoryboardTemplate,
} from "@/lib/storyboardCommands";
import { generateVideoFromText } from "@/lib/videoCommands";
import { useUiStore } from "@/stores/uiStore";
import { useVideoStore } from "@/stores/videoStore";

export function VideoPage() {
  const [prompt, setPrompt] = useState("");
  const [template, setTemplate] = useState<StoryboardTemplate>("commercial");
  const [busy, setBusy] = useState(false);
  const [storyboard, setStoryboard] = useState<Storyboard | null>(null);
  const [selectedSegmentId, setSelectedSegmentId] = useState<string | null>(null);
  const [exportOpen, setExportOpen] = useState(false);
  const [renderResult, setRenderResult] = useState<AssemblyResult | null>(null);
  const [renderBusy, setRenderBusy] = useState(false);
  const segments = useVideoStore((s) => s.segments);
  const addSegment = useVideoStore((s) => s.addSegment);
  const updateSegment = useVideoStore((s) => s.updateSegment);
  const applyVideoResult = useVideoStore((s) => s.applyVideoResult);
  const removeSegment = useVideoStore((s) => s.removeSegment);
  const moveSegment = useVideoStore((s) => s.moveSegment);
  const resetSegments = useVideoStore((s) => s.reset);
  const notify = useUiStore((s) => s.notify);

  // Flow (A): seed segments from storyboard shots if none exist yet.
  useEffect(() => {
    if (!storyboard) return;
    if (useVideoStore.getState().segments.length > 0) return;
    storyboard.shots.forEach((shot) => {
      addSegment({
        kind: "ai",
        label: shot.description || `Shot ${shot.index}`,
        duration_s: shot.duration_s,
      });
    });
  }, [storyboard, addSegment]);

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

  // Flow (B): iterate segments and generate each via the right backend.
  async function generateAllSegments() {
    const current = useVideoStore.getState().segments;
    for (const seg of current) {
      if (seg.video_url || seg.local_path) continue; // skip already-rendered
      updateSegment(seg.id, { busy: true, error: undefined });
      try {
        if (seg.kind === "ai") {
          const result = await generateVideoFromText({
            prompt: seg.label,
            duration_s: seg.duration_s,
            module: "video",
          });
          applyVideoResult(seg.id, result);
        } else if (seg.kind === "remotion") {
          const result = await renderRemotion({
            composition: "KineticTypography",
            props: { text: seg.label },
          });
          updateSegment(seg.id, {
            busy: false,
            local_path: result.output_path,
            video_url: result.output_path,
            model: "Remotion",
            error: undefined,
          });
        } else {
          updateSegment(seg.id, { busy: false });
        }
      } catch (err) {
        const detail = err instanceof Error ? err.message : String(err);
        updateSegment(seg.id, { busy: false, error: detail });
        notify({
          kind: "error",
          message: `Segment "${seg.label}" failed`,
          detail,
        });
      }
    }
  }

  // Flow (C): assemble segments via Shotstack.
  async function handleExport(settings: RenderSettings) {
    const segs = useVideoStore.getState().segments;
    const withUrl = segs.filter((s) => s.video_url);
    if (withUrl.length === 0) {
      notify({
        kind: "warning",
        message: "No rendered segments to assemble",
      });
      return;
    }
    setRenderBusy(true);
    setExportOpen(false);
    try {
      let cursor = 0;
      const clips = withUrl.map((s) => {
        const clip = {
          src: s.video_url as string,
          start_s: cursor,
          length_s: s.duration_s,
        };
        cursor += s.duration_s;
        return clip;
      });
      const result = await assembleVideo({
        clips,
        format: settings.format,
        resolution: settings.resolution,
      });
      setRenderResult(result);
      notify({
        kind: "success",
        message: "Render complete",
        detail: result.video_url,
      });
    } catch (err) {
      notify({
        kind: "error",
        message: "Assembly failed",
        detail: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setRenderBusy(false);
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
              className="rounded-xs border border-neutral-dark-600 bg-neutral-dark-800 px-3 py-2 text-neutral-dark-100 text-sm"
            >
              <option value="commercial">Commercial</option>
              <option value="explainer">Explainer</option>
              <option value="social-media">Social Media</option>
              <option value="music-video">Music Video</option>
              <option value="custom">Custom</option>
            </select>
          </div>
          <LoadingButton
            variant="primary"
            onClick={handleGenerate}
            disabled={!prompt.trim()}
            loading={busy}
          >
            <Sparkles className="h-3 w-3" strokeWidth={1.5} aria-hidden="true" />
            Generate storyboard
          </LoadingButton>
        </div>
      </div>

      {/* Split: toolbar / center (storyboard) / right (segments) */}
      <div className="grid min-h-0 grid-cols-[15rem_1fr_16rem]">
        <div className="flex flex-col gap-3 border-neutral-dark-700 border-r p-4">
          <span className="font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
            Tools
          </span>
          <Button
            variant="secondary"
            size="sm"
            onClick={generateAllSegments}
            disabled={segments.length === 0}
          >
            Generate segments
          </Button>
          <LoadingButton
            variant="primary"
            size="sm"
            onClick={() => setExportOpen(true)}
            disabled={segments.length === 0}
            loading={renderBusy}
          >
            Export video
          </LoadingButton>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              resetSegments();
              setRenderResult(null);
            }}
            disabled={segments.length === 0}
          >
            Clear segments
          </Button>
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
          {renderResult?.local_path ? (
            <div className="border-neutral-dark-700 border-t p-3">
              <span className="mb-2 block font-mono text-2xs text-neutral-dark-400 uppercase tracking-label">
                Rendered
              </span>
              {/* biome-ignore lint/a11y/useMediaCaption: generated videos have no captions */}
              <video
                src={convertFileSrc(renderResult.local_path)}
                controls
                className="w-full rounded-xs border border-neutral-dark-700"
              />
            </div>
          ) : null}
        </div>
      </div>

      {exportOpen ? (
        <RenderExportDialog open onClose={() => setExportOpen(false)} onExport={handleExport} />
      ) : null}
    </div>
  );
}
