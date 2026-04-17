import { AnimatePresence, motion } from "framer-motion";
import { useCallback, useState } from "react";
import { Navigate, Route, Routes, useLocation } from "react-router-dom";
import { NewProjectDialog } from "@/components/projects/NewProjectDialog";
import { Shell } from "@/components/shell/Shell";
import { Toaster } from "@/components/ui/Toast";
import { useBudgetPoll } from "@/hooks/useBudgetPoll";
import { useModuleRouteSync } from "@/hooks/useModuleRouteSync";
import { useProjectsBoot } from "@/hooks/useProjectsBoot";
import { useUndoRedo } from "@/hooks/useUndoRedo";
import { createProject as createProjectCommand, type NewProjectInput } from "@/lib/projectCommands";
import { DesignSystemPage } from "@/pages/DesignSystem";
import { Graphic2DPage } from "@/pages/Graphic2D";
import { ModulePlaceholder } from "@/pages/ModulePlaceholder";
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
          <Route path="/graphic3d" element={<ModulePlaceholder moduleId="graphic3d" />} />
          <Route path="/video" element={<ModulePlaceholder moduleId="video" />} />
          <Route path="/typography" element={<ModulePlaceholder moduleId="typography" />} />
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
  useBudgetPoll();
  const [newDialogOpen, setNewDialogOpen] = useState(false);
  const activeModule = useAppStore((s) => s.activeModule);
  const openProject = useProjectStore((s) => s.openProject);
  const notify = useUiStore((s) => s.notify);

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
      <Toaster />
    </>
  );
}

export default App;
