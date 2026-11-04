import { useEffect } from "react";

interface ToastProps {
  message: string | null;
  onDismiss: () => void;
}

export function Toast({ message, onDismiss }: ToastProps) {
  useEffect(() => {
    if (!message) {
      return;
    }
    const timer = window.setTimeout(onDismiss, 4000);
    return () => window.clearTimeout(timer);
  }, [message, onDismiss]);

  if (!message) {
    return null;
  }

  return (
    <div
      role="status"
      style={{
        position: "fixed",
        left: "50%",
        bottom: 24,
        transform: "translateX(-50%)",
        padding: "10px 16px",
        borderRadius: 6,
        background: "#1f2937",
        color: "#fff",
        fontSize: 14,
        boxShadow: "0 8px 24px rgba(0, 0, 0, 0.2)",
        zIndex: 1000,
      }}
    >
      {message}
    </div>
  );
}
