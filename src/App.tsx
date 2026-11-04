import { useCallback, useEffect, useState } from "react";

import { onStateChanged } from "./api/events";
import { LinkList } from "./components/LinkList";
import { Sidebar } from "./components/Sidebar";
import { Toast } from "./components/Toast";
import { Toolbar } from "./components/Toolbar";
import { useStore } from "./store";
import "./styles/app.css";

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
    <div className="app">
      <Sidebar />
      <div className="main">
        <Toolbar onToast={showToast} />
        <LinkList />
      </div>
      <Toast message={toast} onDismiss={dismissToast} />
    </div>
  );
}

export default App;
