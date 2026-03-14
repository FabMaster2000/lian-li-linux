import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useServerResource } from "./useServerResource";

describe("useServerResource", () => {
  it("loads data and tracks last update timestamps", async () => {
    const load = vi.fn().mockResolvedValue({ value: 1 });
    const { result } = renderHook(() =>
      useServerResource({
        initialData: { value: 0 },
        load,
        loadErrorMessage: "resource load failed",
      }),
    );

    await act(async () => {
      await result.current.refresh();
    });

    expect(load).toHaveBeenCalledTimes(1);
    expect(result.current.data).toEqual({ value: 1 });
    expect(result.current.loading).toBe(false);
    expect(result.current.lastUpdated).not.toBeNull();
  });

  it("supports background refreshes without toggling the primary loading state", async () => {
    let resolveLoad: ((value: { value: number }) => void) | null = null;
    const load = vi
      .fn()
      .mockResolvedValueOnce({ value: 1 })
      .mockImplementationOnce(
        () =>
          new Promise<{ value: number }>((resolve) => {
            resolveLoad = resolve;
          }),
      );
    const { result } = renderHook(() =>
      useServerResource({
        initialData: { value: 1 },
        load,
        loadErrorMessage: "resource load failed",
      }),
    );

    await act(async () => {
      await result.current.refresh();
    });

    act(() => {
      void result.current.refresh({ background: true });
    });

    expect(result.current.loading).toBe(false);
    expect(result.current.refreshing).toBe(true);

    await act(async () => {
      resolveLoad?.({ value: 2 });
    });

    await waitFor(() => expect(result.current.refreshing).toBe(false));
    expect(result.current.data).toEqual({ value: 2 });
  });

  it("stores errors from failed loads", async () => {
    const load = vi.fn().mockRejectedValue(new Error("backend unavailable"));
    const { result } = renderHook(() =>
      useServerResource({
        initialData: { value: 0 },
        load,
        loadErrorMessage: "resource load failed",
      }),
    );

    await act(async () => {
      await result.current.refresh();
    });

    expect(result.current.error).toBe("backend unavailable");
    expect(result.current.loading).toBe(false);
  });
});
