import { Box, Film, Globe, Image, Type } from "lucide-react";
import type { ComponentProps } from "react";
import type { ModuleId } from "@/stores/appStore";

const ICONS = {
  website: Globe,
  graphic2d: Image,
  graphic3d: Box,
  video: Film,
  typography: Type,
} as const;

export interface ModuleIconProps extends Omit<ComponentProps<typeof Globe>, "ref"> {
  moduleId: ModuleId;
}

export function ModuleIcon({ moduleId, className = "", ...rest }: ModuleIconProps) {
  const Icon = ICONS[moduleId];
  return (
    <Icon aria-hidden="true" strokeWidth={1.5} className={`h-3.5 w-3.5 ${className}`} {...rest} />
  );
}
