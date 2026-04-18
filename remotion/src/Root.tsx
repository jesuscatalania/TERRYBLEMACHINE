import { Composition, registerRoot } from "remotion";
import { KineticTypography } from "./compositions/KineticTypography";

export function Root() {
  return (
    <Composition
      id="KineticTypography"
      component={KineticTypography}
      durationInFrames={90}
      fps={30}
      width={1920}
      height={1080}
      defaultProps={{ text: "TERRYBLEMACHINE" }}
    />
  );
}

registerRoot(Root);
