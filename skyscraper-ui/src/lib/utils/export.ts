function triggerDownload(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

export function downloadJson(data: unknown, filename: string) {
  const json = JSON.stringify(data, null, 2)
  triggerDownload(new Blob([json], { type: 'application/json' }), filename)
}

export function downloadCsv(data: Record<string, unknown> | Record<string, unknown>[], filename: string) {
  const rows = Array.isArray(data) ? data : [data]
  if (rows.length === 0) return

  const keys = [...new Set(rows.flatMap((r) => Object.keys(r)))]
  const escape = (v: unknown) => {
    const s = v == null ? '' : String(v)
    return s.includes(',') || s.includes('"') || s.includes('\n')
      ? `"${s.replace(/"/g, '""')}"`
      : s
  }

  const lines = [
    keys.join(','),
    ...rows.map((r) => keys.map((k) => escape(r[k])).join(',')),
  ]
  triggerDownload(new Blob([lines.join('\n')], { type: 'text/csv' }), filename)
}
