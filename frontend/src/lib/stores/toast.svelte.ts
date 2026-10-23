export type ToastTone = 'success' | 'error' | 'info'

class ToastStore {
  message = $state('')
  tone = $state<ToastTone>('success')
  private timer: ReturnType<typeof setTimeout> | null = null

  show(message: string, tone: ToastTone = 'success'): void {
    this.message = message
    this.tone = tone
    if (this.timer) clearTimeout(this.timer)
    this.timer = setTimeout(() => this.dismiss(), 4200)
  }

  success(message: string): void {
    this.show(message, 'success')
  }

  error(message: string): void {
    this.show(message, 'error')
  }

  dismiss(): void {
    this.message = ''
    if (this.timer) {
      clearTimeout(this.timer)
      this.timer = null
    }
  }
}

export const toast = new ToastStore()
