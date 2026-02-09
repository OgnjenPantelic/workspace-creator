import { useRef, useCallback } from "react";

/**
 * Hook for managing SSO polling intervals
 * Used by AWS SSO and Azure CLI login flows
 */
export function useSsoPolling() {
  const ssoPollingRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const isMountedRef = useRef(true);

  const clearSsoPolling = useCallback(() => {
    if (ssoPollingRef.current) {
      clearInterval(ssoPollingRef.current);
      ssoPollingRef.current = null;
    }
  }, []);

  const startPolling = useCallback(
    (
      checkFn: () => Promise<boolean>,
      onSuccess: () => void,
      onTimeout: () => void,
      options: {
        interval: number;
        maxAttempts: number;
      }
    ) => {
      clearSsoPolling();
      let attempts = 0;

      ssoPollingRef.current = setInterval(async () => {
        attempts++;
        try {
          const success = await checkFn();
          if (success) {
            clearSsoPolling();
            if (isMountedRef.current) {
              onSuccess();
            }
          }
        } catch {
          if (attempts >= options.maxAttempts) {
            clearSsoPolling();
            if (isMountedRef.current) {
              onTimeout();
            }
          }
        }
      }, options.interval);
    },
    [clearSsoPolling]
  );

  const cleanup = useCallback(() => {
    isMountedRef.current = false;
    clearSsoPolling();
  }, [clearSsoPolling]);

  const setMounted = useCallback((mounted: boolean) => {
    isMountedRef.current = mounted;
  }, []);

  return {
    startPolling,
    clearSsoPolling,
    cleanup,
    setMounted,
    isMountedRef,
  };
}
