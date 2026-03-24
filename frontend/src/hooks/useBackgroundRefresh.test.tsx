import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useBackgroundRefresh } from "./useBackgroundRefresh";

afterEach(() => {
  vi.useRealTimers();
});

describe("useBackgroundRefresh", () => {
  it("calls the latest refresh callback on the configured interval", async () => {
    vi.useFakeTimers();
    const refresh = vi.fn().mockResolvedValue(undefined);

    renderHook(() => useBackgroundRefresh(refresh, 5000));

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });

    expect(refresh).toHaveBeenCalledTimes(1);
  });

  it("skips overlapping refresh calls until the previous one settles", async () => {
    vi.useFakeTimers();
    let resolveRefresh: (() => void) | null = null;
    const refresh = vi.fn().mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          resolveRefresh = resolve;
        }),
    );

    renderHook(() => useBackgroundRefresh(refresh, 5000));

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });
    expect(refresh).toHaveBeenCalledTimes(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });
    expect(refresh).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveRefresh?.();
      await Promise.resolve();
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });
    expect(refresh).toHaveBeenCalledTimes(2);
  });
});
