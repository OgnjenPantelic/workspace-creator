import { useState, useCallback } from "react";

/**
 * A boolean state hook that persists its value to localStorage under `cfg_<key>`.
 * Returns `[value, setValue]` like useState, but the setter also writes to localStorage.
 */
export function usePersistedCollapse(
  key: string,
  defaultValue: boolean,
): [boolean, (v: boolean) => void] {
  const [value, _setValue] = useState(() => {
    try {
      const stored = localStorage.getItem(`cfg_${key}`);
      return stored !== null ? stored === "true" : defaultValue;
    } catch {
      return defaultValue;
    }
  });

  const setValue = useCallback(
    (v: boolean) => {
      _setValue(v);
      try {
        localStorage.setItem(`cfg_${key}`, String(v));
      } catch {
        /* noop */
      }
    },
    [key],
  );

  return [value, setValue];
}
