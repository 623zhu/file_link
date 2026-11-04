import { useCallback, useEffect, useState } from "react";

import { onStateChanged } from "./api/events";
import { Toast } from "./components/Toast";
import { Toolbar } from "./components/Toolbar";
import { useStore } from "./store";

function App() {
  const refresh = useStore((s) => s.refresh);
  const [toast, setToast] = useState<string | null>(null);
  const showToast = useCallback((message: string) => {
    setToast(message);
  }, []);
  const dismissToast = useCallback(() => {
    setToast(null);
  }, []);

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

  return (
    <div style={{ fontFamily: '"Segoe UI", sans-serif' }}>
      <Toolbar onToast={showToast} />
      <Toast message={toast} onDismiss={dismissToast} />
    </div>
  );
}

export default App;
