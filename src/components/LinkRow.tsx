import type { Binding, Link, Source } from "../types";

interface LinkRowProps {
  link: Link;
  bindings: Binding[];
}

export function LinkRow({ link, bindings }: LinkRowProps) {
  return (
    <div className={link.missing ? "link-row missing" : "link-row"}>
      <div className="link-cell link-name">
        <span className="link-name-text" title={link.name}>
          {link.name}
        </span>
        {link.missing ? <span className="missing-badge">缺失</span> : null}
      </div>
      <div className="link-cell" title={link.path}>
        {link.path}
      </div>
      <div className="link-cell">{sourceLabel(link.sources, bindings)}</div>
      <div className="link-cell">{formatAddedAt(link.addedAt)}</div>
    </div>
  );
}

function sourceLabel(sources: Source[], bindings: Binding[]): string {
  const labels: string[] = [];
  for (const source of sources) {
    if (source.type === "manual") {
      labels.push("手动");
      continue;
    }
    const binding = bindings.find((item) => item.id === source.bindingId);
    labels.push(folderName(binding?.path) ?? "文件夹");
  }
  return labels.join(" ");
}

function folderName(path: string | undefined): string | undefined {
  if (!path) {
    return undefined;
  }
  const parts = path.split(/[\\/]/).filter((part) => part.length > 0);
  return parts[parts.length - 1];
}

function formatAddedAt(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  return `${month}-${day} ${hours}:${minutes}`;
}
