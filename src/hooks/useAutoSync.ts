import {useEffect, useRef} from 'react';

/** Ruft `run` alle `hours` Stunden auf, solange `enabled` gilt (App muss laufen). */
export function useAutoSync(enabled: boolean, hours: number, run: () => void | Promise<void>) {
  const latest = useRef(run);
  latest.current = run;
  useEffect(() => {
    if (!enabled || hours <= 0) return;
    const id = setInterval(() => void latest.current(), hours * 3_600_000);
    return () => clearInterval(id);
  }, [enabled, hours]);
}
