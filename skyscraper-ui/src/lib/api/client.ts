const BASE_URL = '/api/v1'

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message)
  }
}

async function handleResponse<T>(res: Response): Promise<T> {
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }))
    throw new ApiError(res.status, body.error || res.statusText)
  }
  if (res.status === 204 || res.headers.get('content-length') === '0') {
    return {} as T
  }
  return res.json()
}

function headers(apiKey?: string): HeadersInit {
  const h: HeadersInit = { 'Content-Type': 'application/json' }
  if (apiKey) h['X-Api-Key'] = apiKey
  return h
}

export async function get<T>(path: string, apiKey?: string): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, { headers: headers(apiKey) })
  return handleResponse<T>(res)
}

export async function post<T>(path: string, body: unknown, apiKey?: string): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method: 'POST',
    headers: headers(apiKey),
    body: JSON.stringify(body),
  })
  return handleResponse<T>(res)
}

export async function patch<T>(path: string, body: unknown, apiKey?: string): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method: 'PATCH',
    headers: headers(apiKey),
    body: JSON.stringify(body),
  })
  return handleResponse<T>(res)
}

export async function del<T>(path: string, apiKey?: string): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method: 'DELETE',
    headers: headers(apiKey),
  })
  return handleResponse<T>(res)
}

export async function fetchExternal<T>(url: string, apiKey?: string): Promise<T> {
  const res = await fetch(url, { headers: apiKey ? { 'X-Api-Key': apiKey } : {} })
  return handleResponse<T>(res)
}
