import { AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig } from "remotion";

export interface KineticTypographyProps {
  text: string;
}

export function KineticTypography({ text }: KineticTypographyProps) {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const opacity = interpolate(
    frame,
    [0, 15, durationInFrames - 15, durationInFrames],
    [0, 1, 1, 0],
  );
  const scale = interpolate(frame, [0, 30], [0.95, 1], { extrapolateRight: "clamp" });
  return (
    <AbsoluteFill
      style={{
        backgroundColor: "#0E0E11",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <div
        style={{
          color: "#F7F7F8",
          fontFamily: "Inter, sans-serif",
          fontSize: 140,
          fontWeight: 700,
          letterSpacing: -3,
          opacity,
          transform: `scale(${scale})`,
        }}
      >
        {text}
      </div>
    </AbsoluteFill>
  );
}
