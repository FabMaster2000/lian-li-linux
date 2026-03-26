import { BrowserRouter, Navigate, Route, Routes, useLocation } from "react-router-dom";
import { AppLayout } from "./AppLayout";
import { BackendEventsProvider } from "./BackendEventsProvider";
import { SystemStatusProvider } from "./SystemStatusProvider";
import { DashboardPage } from "../pages/DashboardPage";
import { LightingPage } from "../pages/LightingPage";
import { FansPage } from "../pages/FansPage";
import { WirelessSyncPage } from "../pages/WirelessSyncPage";
import { DevicesPage } from "../pages/DevicesPage";
import { DeviceDetailPage } from "../pages/DeviceDetailPage";

function LegacyLightingRedirect() {
  const location = useLocation();

  return <Navigate replace to={{ pathname: "/rgb", search: location.search }} />;
}

export default function App() {
  return (
    <BrowserRouter>
      <BackendEventsProvider>
        <SystemStatusProvider>
          <Routes>
            <Route element={<AppLayout />}>
              <Route index element={<DashboardPage />} />
              <Route path="/rgb" element={<LightingPage />} />
              <Route path="/rgb-effects" element={<Navigate replace to="/rgb" />} />
              <Route path="/fans" element={<FansPage />} />
              <Route path="/wireless-sync" element={<WirelessSyncPage />} />
              <Route path="/devices" element={<DevicesPage />} />
              <Route path="/devices/:deviceId" element={<DeviceDetailPage />} />
              <Route path="/lighting" element={<LegacyLightingRedirect />} />
              <Route path="/lcd-media" element={<Navigate replace to="/" />} />
              <Route path="*" element={<Navigate replace to="/" />} />
            </Route>
          </Routes>
        </SystemStatusProvider>
      </BackendEventsProvider>
    </BrowserRouter>
  );
}
