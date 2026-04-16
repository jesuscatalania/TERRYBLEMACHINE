import { Route, Routes } from "react-router-dom";
import { Shell } from "@/components/shell/Shell";
import { Toaster } from "@/components/ui/Toast";
import { DesignSystemPage } from "@/pages/DesignSystem";
import { HomePage } from "@/pages/Home";

function App() {
  return (
    <>
      <Shell>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/design-system" element={<DesignSystemPage />} />
        </Routes>
      </Shell>
      <Toaster />
    </>
  );
}

export default App;
