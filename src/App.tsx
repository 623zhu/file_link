import { useCallback, useEffect, useState } from "react";

import { subscribeWatcherEvents } from "./api/events";
import { LinkList } from "./components/LinkList";
import { Sidebar } from "./components/Sidebar";
import { Toast } from "./components/Toast";
import { Toolbar } from "./components/Toolbar";
import { useStore } from "./store";
import { autoImportedMessage } from "./utils/path";
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

    void subscribeWatcherEvents({
      onStateChanged: () => {
        void refresh();
      },
      onAutoImported: (payload) => {
        const binding = useStore
          .getState()
          .snapshot?.bindings.find((item) => item.id === payload.bindingId);
        showToast(autoImportedMessage(binding?.path, payload.added));
      },
      onBindingUnavailable: (payload) => {
        const snapshot = useStore.getState().snapshot;
        if (snapshot) {
          useStore.setState({
            snapshot: {
              ...snapshot,
              bindings: snapshot.bindings.map((binding) =>
                binding.id === payload.bindingId
                  ? { ...binding, unavailable: true }
                  : binding,
              ),
            },
          });
        }
        void refresh();
      },
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
  }, [refresh, showToast]);

  return (
    <div className="app">
      <Sidebar onToast={showToast} />
      <div className="main">
        <Toolbar onToast={showToast} />
        <LinkList onToast={showToast} />
      </div>
      <Toast message={toast} onDismiss={dismissToast} />
    </div>
  );
}

export default App;
