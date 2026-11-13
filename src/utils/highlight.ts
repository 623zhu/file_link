export interface HighlightPart {
  text: string;
  hit: boolean;
}

export function splitHighlight(text: string, query: string): HighlightPart[] {
  const needle = query.trim().toLowerCase();
  const haystack = text.toLowerCase();
  // toLowerCase can change length for a few characters; indexes would no longer line up.
  if (needle.length === 0 || haystack.length !== text.length) {
    return [{ text, hit: false }];
  }
  const parts: HighlightPart[] = [];
  let start = 0;
  let index = haystack.indexOf(needle);
  while (index >= 0) {
    if (index > start) {
      parts.push({ text: text.slice(start, index), hit: false });
    }
    parts.push({ text: text.slice(index, index + needle.length), hit: true });
    start = index + needle.length;
    index = haystack.indexOf(needle, start);
  }
  if (start < text.length || parts.length === 0) {
    parts.push({ text: text.slice(start), hit: false });
  }
  return parts;
}
