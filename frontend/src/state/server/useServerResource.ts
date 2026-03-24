import { useCallback, useRef, useState } from "react";

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
  const loadRef = useRef(load);
  const loadErrorMessageRef = useRef(loadErrorMessage);
  const inFlightRef = useRef<Promise<TData | null> | null>(null);

  loadRef.current = load;
  loadErrorMessageRef.current = loadErrorMessage;

  const refresh = useCallback(
    async (options: RefreshOptions = {}) => {
      const { background = false } = options;

      if (background) {
        setRefreshing(true);
      } else {
        setLoading(true);
      }

      setError(null);

      let request = inFlightRef.current;

      if (!request) {
        request = loadRef.current()
          .then((nextData) => {
            setData(nextData);
            setLastUpdated(new Date().toISOString());
            return nextData;
          })
          .catch((nextError) => {
            setError(toErrorMessage(nextError, loadErrorMessageRef.current));
            return null;
          })
          .finally(() => {
            if (inFlightRef.current === request) {
              inFlightRef.current = null;
            }
          });

        inFlightRef.current = request;
      }

      try {
        return await request;
      } finally {
        if (background) {
          setRefreshing(false);
        } else {
          setLoading(false);
        }
      }
    },
    [],
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
