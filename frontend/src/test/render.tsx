import type { PropsWithChildren, ReactElement } from "react";
import { render } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";

type RenderRouteOptions = {
  initialPath?: string;
  routePath?: string;
};

export const routerFuture = {
  v7_startTransition: true,
  v7_relativeSplatPath: true,
} as const;

export function renderAtRoute(
  element: ReactElement,
  options: RenderRouteOptions = {},
) {
  const { initialPath = "/", routePath = "*" } = options;

  function Wrapper({ children }: PropsWithChildren) {
    return (
      <MemoryRouter future={routerFuture} initialEntries={[initialPath]}>
        <Routes>
          <Route element={children} path={routePath} />
        </Routes>
      </MemoryRouter>
    );
  }

  return render(element, { wrapper: Wrapper });
}
