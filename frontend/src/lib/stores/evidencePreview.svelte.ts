import type { EvidenceItem } from '../types'

class EvidencePreviewStore {
  item = $state<EvidenceItem | null>(null)

  open(item: EvidenceItem): void {
    this.item = item
  }

  close(): void {
    this.item = null
  }
}

export const evidencePreview = new EvidencePreviewStore()
