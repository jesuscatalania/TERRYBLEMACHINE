import { Composition, registerRoot } from "remotion";
import { KineticTypography } from "./compositions/KineticTypography";
import { MotionGraphics } from "./compositions/MotionGraphics";

export function Root() {
  return (
    <>
      <Composition
        id="KineticTypography"
        component={KineticTypography}
        durationInFrames={90}
        fps={30}
        width={1920}
        height={1080}
        defaultProps={{ text: "TERRYBLEMACHINE" }}
      />
      <Composition
        id="MotionGraphics"
        component={MotionGraphics}
        durationInFrames={180}
        fps={30}
        width={1920}
        height={1080}
        defaultProps={{ title: "Revenue Growth", value: 1247 }}
      />
    </>
  );
}

registerRoot(Root);
