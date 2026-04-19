import { AnimatePresence, motion } from "framer-motion";
import { useCallback, useState } from "react";
import { Navigate, Route, Routes, useLocation } from "react-router-dom";
import { WelcomeModal } from "@/components/onboarding/WelcomeModal";
import { NewProjectDialog } from "@/components/projects/NewProjectDialog";
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
import { DesignSystemPage } from "@/pages/DesignSystem";
import { Graphic2DPage } from "@/pages/Graphic2D";
import { Graphic3DPage } from "@/pages/Graphic3D";
import { TypographyPage } from "@/pages/Typography";
import { VideoPage } from "@/pages/Video";
import { WebsiteBuilderPage } from "@/pages/WebsiteBuilder";
import { useAppStore } from "@/stores/appStore";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

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
      <Shell onNew={() => setNewDialogOpen(true)}>
        <AnimatedRoutes />
      </Shell>
      <NewProjectDialog
        open={newDialogOpen}
        onClose={() => setNewDialogOpen(false)}
        onCreate={handleCreate}
        defaultModule={activeModule}
      />
      <WelcomeModal />
      <ShortcutHelpOverlay open={helpOpen} onClose={closeHelp} />
      <Toaster />
    </>
  );
}

export default App;
