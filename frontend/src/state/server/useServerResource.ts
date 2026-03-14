import { useCallback, useState } from "react";

type RefreshOptions = {
  background?: boolean;
};

type UseServerResourceOptions<TData> = {
  initialData: TData;
  load: () => Promise<TData>;
  loadErrorMessage: string;
};

function toErrorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

export function useServerResource<TData>({
  initialData,
  load,
  loadErrorMessage,
}: UseServerResourceOptions<TData>) {
  const [data, setData] = useState<TData>(initialData);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<string | null>(null);

  const refresh = useCallback(
    async (options: RefreshOptions = {}) => {
      const { background = false } = options;

      if (background) {
        setRefreshing(true);
      } else {
        setLoading(true);
      }

      setError(null);

      try {
        const nextData = await load();
        setData(nextData);
        setLastUpdated(new Date().toISOString());
        return nextData;
      } catch (error) {
        setError(toErrorMessage(error, loadErrorMessage));
        return null;
      } finally {
        if (background) {
          setRefreshing(false);
        } else {
          setLoading(false);
        }
      }
    },
    [load, loadErrorMessage],
  );

  return {
    data,
    loading,
    refreshing,
    error,
    lastUpdated,
    refresh,
  };
}
