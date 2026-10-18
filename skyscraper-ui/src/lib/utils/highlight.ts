export function highlightJson(obj: unknown): string {
  return JSON.stringify(obj, null, 2)
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/"([^"]+)":/g, '<span class="text-accent">"$1"</span>:')
    .replace(/: "([^"]*)"/g, ': <span class="text-success">"$1"</span>')
    .replace(/: (\d+\.?\d*)/g, ': <span class="text-warning">$1</span>')
    .replace(/: (true|false)/g, ': <span class="text-info">$1</span>')
    .replace(/: (null)/g, ': <span class="text-text-muted">$1</span>')
}

export function highlightHtml(raw: string): string {
  const t = raw.length > 50000 ? raw.slice(0, 50000) + '\n...' : raw
  return t.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/&lt;(\/?)([\w-]+)/g, '&lt;$1<span class="text-accent">$2</span>')
    .replace(/ ([\w-]+)="/g, ' <span class="text-warning">$1</span>="')
}
