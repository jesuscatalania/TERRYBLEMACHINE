import { AnimatePresence, motion } from "framer-motion";
import { Navigate, Route, Routes, useLocation } from "react-router-dom";
import { Shell } from "@/components/shell/Shell";
import { Toaster } from "@/components/ui/Toast";
import { useModuleRouteSync } from "@/hooks/useModuleRouteSync";
import { DesignSystemPage } from "@/pages/DesignSystem";
import { ModulePlaceholder } from "@/pages/ModulePlaceholder";

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
          <Route path="/website" element={<ModulePlaceholder moduleId="website" />} />
          <Route path="/graphic2d" element={<ModulePlaceholder moduleId="graphic2d" />} />
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
  return (
    <>
      <Shell>
        <AnimatedRoutes />
      </Shell>
      <Toaster />
    </>
  );
}

export default App;
