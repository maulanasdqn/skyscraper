const JUNK_TAGS = ['style', 'svg', 'noscript', 'link', 'meta', 'iframe', 'video', 'audio']

export function extractStructuredData(raw: string): Record<string, unknown> | null {
  const ldScripts = raw.match(/<script[^>]*type\s*=\s*["']application\/ld\+json["'][^>]*>([\s\S]*?)<\/script>/gi)
  if (!ldScripts?.length) return null
  const results: unknown[] = []
  for (const s of ldScripts) {
    const content = s.replace(/<\/?script[^>]*>/gi, '').trim()
    try {
      const parsed = JSON.parse(content)
      if (Array.isArray(parsed)) results.push(...parsed)
      else results.push(parsed)
    } catch {}
  }
  if (!results.length) return null
  const product = results.find((r: any) => r['@type'] === 'Product') as Record<string, unknown> | undefined
  if (product) return product
  return results.length === 1 ? results[0] as Record<string, unknown> : { items: results }
}

export function extractPageText(raw: string): string {
  const parser = new DOMParser()
  const doc = parser.parseFromString(raw, 'text/html')
  JUNK_TAGS.forEach((tag) => doc.querySelectorAll(tag).forEach((el) => el.remove()))
  doc.querySelectorAll('script').forEach((el) => el.remove())
  doc.querySelectorAll('[aria-hidden="true"], [hidden]').forEach((el) => el.remove())

  const parts: string[] = []
  const title = doc.querySelector('title')?.textContent?.trim()
  if (title) parts.push(`Page title: ${title}`)

  const og: Record<string, string> = {}
  doc.querySelectorAll('meta[property^="og:"], meta[property^="product:"]').forEach((el) => {
    const prop = el.getAttribute('property')
    const val = el.getAttribute('content')
    if (prop && val) og[prop] = val
  })
  if (Object.keys(og).length) parts.push('OpenGraph: ' + JSON.stringify(og))

  const selectors = [
    '[data-testid*="price"], [data-testid*="Price"]',
    '[class*="price"], [class*="Price"]',
    '[itemprop="price"], [itemprop="priceCurrency"]',
    '[class*="rating"], [class*="Rating"], [itemprop="ratingValue"]',
    '[class*="product-title"], [class*="productTitle"], [itemprop="name"]',
    'h1',
  ]
  const extracted: string[] = []
  for (const sel of selectors) {
    doc.querySelectorAll(sel).forEach((el) => {
      const t = el.textContent?.replace(/\s+/g, ' ').trim()
      if (t && t.length < 500 && !extracted.includes(t)) extracted.push(t)
    })
  }
  if (extracted.length) parts.push('Key elements: ' + extracted.join(' | '))

  const body = (doc.body?.textContent || '').replace(/\s+/g, ' ').trim()
  parts.push(body.slice(0, 8000))
  return parts.join('\n\n')
}
