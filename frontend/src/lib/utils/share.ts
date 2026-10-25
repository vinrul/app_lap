export function presentationLink(token: string): string {
  const base = `${window.location.origin}${window.location.pathname}`
  return `${base}#/present/${encodeURIComponent(token)}`
}

export function publicPdfLink(token: string): string {
  return `${window.location.origin}/api/public/shares/${encodeURIComponent(token)}/pdf`
}

export async function copyText(value: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(value)
      return true
    }
  } catch {
    // Clipboard API dapat ditolak pada origin/http tertentu; pakai fallback.
  }

  try {
    const field = document.createElement('textarea')
    field.value = value
    field.setAttribute('readonly', '')
    field.style.position = 'fixed'
    field.style.opacity = '0'
    field.style.pointerEvents = 'none'
    document.body.appendChild(field)
    field.select()
    const copied = document.execCommand('copy')
    field.remove()
    return copied
  } catch {
    return false
  }
}
