import { Config } from "@remotion/cli/config";

Config.setVideoImageFormat("jpeg");
Config.setOverwriteOutput(true);
// GPU acceleration for M-series: ANGLE is Apple's Metal->OpenGL translation layer
Config.setChromiumOpenGlRenderer("angle");
