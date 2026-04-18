import { AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";

export interface MotionGraphicsProps {
  title: string;
  value: number;
}

export function MotionGraphics({ title, value }: MotionGraphicsProps) {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();

  // Rectangle slide-in via spring
  const rectX = spring({ fps, frame, config: { damping: 12 } });

  // Count-up for the number
  const countFrame = Math.min(frame, durationInFrames - 30);
  const displayed = Math.round(
    interpolate(countFrame, [0, durationInFrames - 30], [0, value], {
      extrapolateRight: "clamp",
    }),
  );

  return (
    <AbsoluteFill style={{ backgroundColor: "#0E0E11" }}>
      <div
        style={{
          position: "absolute",
          left: 100,
          top: 120,
          width: 960 * rectX,
          height: 16,
          backgroundColor: "#e85d2d",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: 100,
          top: 180,
          color: "#F7F7F8",
          fontFamily: "Inter, sans-serif",
          fontSize: 80,
          fontWeight: 600,
        }}
      >
        {title}
      </div>
      <div
        style={{
          position: "absolute",
          left: 100,
          top: 320,
          color: "#e85d2d",
          fontFamily: "IBM Plex Mono, monospace",
          fontSize: 240,
          fontWeight: 500,
          letterSpacing: -6,
        }}
      >
        {displayed.toLocaleString()}
      </div>
    </AbsoluteFill>
  );
}
