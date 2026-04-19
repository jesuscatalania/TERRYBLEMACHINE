import { AnimatePresence, motion } from "framer-motion";
import { lazy, Suspense, useCallback, useState } from "react";
import { Navigate, Route, Routes, useLocation } from "react-router-dom";
import { WelcomeModal } from "@/components/onboarding/WelcomeModal";
import { NewProjectDialog } from "@/components/projects/NewProjectDialog";
import { SettingsModal } from "@/components/settings/SettingsModal";
import { ModuleLoadingFallback } from "@/components/shell/ModuleLoadingFallback";
import { Shell } from "@/components/shell/Shell";
import { ShortcutHelpOverlay } from "@/components/shell/ShortcutHelpOverlay";
import { Toaster } from "@/components/ui/Toast";
import { useBudgetPoll } from "@/hooks/useBudgetPoll";
import { useGlobalKeyboardDispatch } from "@/hooks/useGlobalKeyboardDispatch";
import { useKeyboardShortcut } from "@/hooks/useKeyboardShortcut";
import { useModuleRouteSync } from "@/hooks/useModuleRouteSync";
import { useProjectsBoot } from "@/hooks/useProjectsBoot";
import { useUndoRedo } from "@/hooks/useUndoRedo";
import { createProject as createProjectCommand, type NewProjectInput } from "@/lib/projectCommands";
import { useAppStore } from "@/stores/appStore";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

const DesignSystemPage = lazy(() =>
  import("@/pages/DesignSystem").then((m) => ({ default: m.DesignSystemPage })),
);
const Graphic2DPage = lazy(() =>
  import("@/pages/Graphic2D").then((m) => ({ default: m.Graphic2DPage })),
);
const Graphic3DPage = lazy(() =>
  import("@/pages/Graphic3D").then((m) => ({ default: m.Graphic3DPage })),
);
const TypographyPage = lazy(() =>
  import("@/pages/Typography").then((m) => ({ default: m.TypographyPage })),
);
const VideoPage = lazy(() => import("@/pages/Video").then((m) => ({ default: m.VideoPage })));
const WebsiteBuilderPage = lazy(() =>
  import("@/pages/WebsiteBuilder").then((m) => ({ default: m.WebsiteBuilderPage })),
);

function AnimatedRoutes() {
  const location = useLocation();
  useModuleRouteSync();

  return (
    <AnimatePresence mode="wait" initial={false}>
      <motion.div
        key={location.pathname}
        initial={{ opacity: 0, y: 4 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -4 }}
        transition={{ duration: 0.14, ease: "easeOut" }}
        className="h-full"
      >
        <Suspense fallback={<ModuleLoadingFallback />}>
          <Routes location={location}>
            <Route path="/" element={<Navigate to="/website" replace />} />
            <Route path="/website" element={<WebsiteBuilderPage />} />
            <Route path="/graphic2d" element={<Graphic2DPage />} />
            <Route path="/graphic3d" element={<Graphic3DPage />} />
            <Route path="/video" element={<VideoPage />} />
            <Route path="/typography" element={<TypographyPage />} />
            <Route path="/design-system" element={<DesignSystemPage />} />
            <Route path="*" element={<Navigate to="/website" replace />} />
          </Routes>
        </Suspense>
      </motion.div>
    </AnimatePresence>
  );
}

function App() {
  useProjectsBoot();
  useUndoRedo();
  useGlobalKeyboardDispatch();
  useBudgetPoll();
  const [newDialogOpen, setNewDialogOpen] = useState(false);
  const activeModule = useAppStore((s) => s.activeModule);
  const openProject = useProjectStore((s) => s.openProject);
  const notify = useUiStore((s) => s.notify);

  const openNewProject = useCallback(() => setNewDialogOpen(true), []);
  useKeyboardShortcut({
    id: "global:new-project",
    combo: "Mod+N",
    handler: openNewProject,
    scope: "global",
    label: "New project",
  });

  const [settingsOpen, setSettingsOpen] = useState(false);
  const openSettings = useCallback(() => setSettingsOpen(true), []);
  const closeSettings = useCallback(() => setSettingsOpen(false), []);
  useKeyboardShortcut({
    id: "global:settings",
    combo: "Mod+,",
    handler: openSettings,
    scope: "global",
    label: "Settings",
  });

  const [helpOpen, setHelpOpen] = useState(false);
  const toggleHelp = useCallback(() => setHelpOpen((v) => !v), []);
  useKeyboardShortcut({
    id: "global:help",
    combo: "Mod+/",
    handler: toggleHelp,
    scope: "global",
    label: "Keyboard shortcuts",
  });
  useKeyboardShortcut({
    id: "global:help-q",
    combo: "?",
    handler: toggleHelp,
    scope: "global",
    label: "Keyboard shortcuts (?)",
  });
  const closeHelp = useCallback(() => setHelpOpen(false), []);

  const handleCreate = useCallback(
    async (input: NewProjectInput) => {
      const created = await createProjectCommand(input);
      openProject(created);
      notify({
        kind: "success",
        message: `Project "${created.name}" created`,
        detail: created.path,
      });
    },
    [openProject, notify],
  );

  return (
    <>
      <Shell onNew={() => setNewDialogOpen(true)} onOpenSettings={openSettings}>
        <AnimatedRoutes />
      </Shell>
      <NewProjectDialog
        open={newDialogOpen}
        onClose={() => setNewDialogOpen(false)}
        onCreate={handleCreate}
        defaultModule={activeModule}
      />
      <SettingsModal open={settingsOpen} onClose={closeSettings} />
      <WelcomeModal />
      <ShortcutHelpOverlay open={helpOpen} onClose={closeHelp} />
      <Toaster />
    </>
  );
}

export default App;
