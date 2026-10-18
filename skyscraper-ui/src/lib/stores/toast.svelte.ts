type ToastType = 'success' | 'error' | 'info'
type Toast = { id: number; type: ToastType; message: string }

let toasts = $state<Toast[]>([])
let nextId = 0

export function getToasts(): Toast[] { return toasts }

export function showToast(type: ToastType, message: string, durationMs = 4000) {
  const id = nextId++
  toasts = [...toasts, { id, type, message }]
  setTimeout(() => dismissToast(id), durationMs)
}

export function dismissToast(id: number) {
  toasts = toasts.filter(t => t.id !== id)
}
