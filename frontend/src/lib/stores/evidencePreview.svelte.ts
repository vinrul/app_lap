import type { EvidenceItem } from '../types'

class EvidencePreviewStore {
  items = $state<EvidenceItem[]>([])
  index = $state(0)

  get item(): EvidenceItem | null {
    return this.items[this.index] ?? null
  }

  get hasMultiple(): boolean {
    return this.items.length > 1
  }

  get position(): number {
    return this.items.length ? this.index + 1 : 0
  }

  open(item: EvidenceItem, items: EvidenceItem[] = [item]): void {
    const ready = items.filter((entry) => entry.status === 'ready')
    const list = ready.some((entry) => entry.id === item.id) ? ready : [item]
    const index = list.findIndex((entry) => entry.id === item.id)
    this.items = list
    this.index = index >= 0 ? index : 0
  }

  select(index: number): void {
    if (index >= 0 && index < this.items.length) {
      this.index = index
    }
  }

  next(): void {
    if (this.items.length) {
      this.index = (this.index + 1) % this.items.length
    }
  }

  prev(): void {
    if (this.items.length) {
      this.index = (this.index - 1 + this.items.length) % this.items.length
    }
  }

  close(): void {
    this.items = []
    this.index = 0
  }
}

export const evidencePreview = new EvidencePreviewStore()
