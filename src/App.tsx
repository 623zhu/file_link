import { useEffect } from "react";

import { onStateChanged } from "./api/events";
import { useStore } from "./store";

function App() {
  const refresh = useStore((s) => s.refresh);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void refresh();

    void onStateChanged(() => {
      void refresh();
    }).then((fn) => {
      if (cancelled) {
        fn();
        return;
      }
      unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [refresh]);

  return null;
}

export default App;
