import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "./AppLayout";
import { BackendEventsProvider } from "./BackendEventsProvider";
import { SystemStatusProvider } from "./SystemStatusProvider";
import { DashboardPage } from "../pages/DashboardPage";
import { DevicesPage } from "../pages/DevicesPage";
import { DeviceDetailPage } from "../pages/DeviceDetailPage";
import { LightingPage } from "../pages/LightingPage";
import { FansPage } from "../pages/FansPage";
import { ProfilesPage } from "../pages/ProfilesPage";
import { SettingsPage } from "../pages/SettingsPage";

export default function App() {
  return (
    <BrowserRouter>
      <BackendEventsProvider>
        <SystemStatusProvider>
          <Routes>
            <Route element={<AppLayout />}>
              <Route index element={<DashboardPage />} />
              <Route path="/devices" element={<DevicesPage />} />
              <Route path="/devices/:deviceId" element={<DeviceDetailPage />} />
              <Route path="/lighting" element={<LightingPage />} />
              <Route path="/fans" element={<FansPage />} />
              <Route path="/profiles" element={<ProfilesPage />} />
              <Route path="/settings" element={<SettingsPage />} />
              <Route path="*" element={<Navigate replace to="/" />} />
            </Route>
          </Routes>
        </SystemStatusProvider>
      </BackendEventsProvider>
    </BrowserRouter>
  );
}
