export type DialogTone = 'default' | 'danger'

export interface ConfirmOptions {
  title: string
  message?: string
  confirmLabel?: string
  cancelLabel?: string
  tone?: DialogTone
}

export interface PromptOptions extends ConfirmOptions {
  inputLabel?: string
  inputPlaceholder?: string
  inputValue?: string
  inputType?: 'text' | 'url'
  inputHint?: string
  required?: boolean
}

type DialogRequest =
  | { kind: 'confirm'; options: ConfirmOptions }
  | { kind: 'prompt'; options: PromptOptions }

class DialogStore {
  request = $state<DialogRequest | null>(null)
  value = $state('')
  private resolver: ((result: string | boolean | null) => void) | null = null

  confirm(options: ConfirmOptions): Promise<boolean> {
    this.closeCurrent(null)
    this.request = { kind: 'confirm', options }
    return new Promise<boolean>((resolve) => {
      this.resolver = (result) => resolve(result === true)
    })
  }

  prompt(options: PromptOptions): Promise<string | null> {
    this.closeCurrent(null)
    this.value = options.inputValue ?? ''
    this.request = { kind: 'prompt', options }
    return new Promise<string | null>((resolve) => {
      this.resolver = (result) => resolve(typeof result === 'string' ? result : null)
    })
  }

  accept(): void {
    const request = this.request
    if (!request) return
    if (request.kind === 'prompt') {
      const value = this.value.trim()
      if (request.options.required !== false && !value) return
      this.finish(value)
      return
    }
    this.finish(true)
  }

  cancel(): void {
    this.finish(null)
  }

  update(value: string): void {
    this.value = value
  }

  private finish(result: string | boolean | null): void {
    const resolver = this.resolver
    this.resolver = null
    this.request = null
    this.value = ''
    resolver?.(result)
  }

  private closeCurrent(result: string | boolean | null): void {
    if (!this.resolver) return
    const resolver = this.resolver
    this.resolver = null
    this.request = null
    this.value = ''
    resolver(result)
  }
}

export const dialog = new DialogStore()
