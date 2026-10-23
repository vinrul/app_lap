import { metaApi } from '../api'
import type { Meta } from '../types'

class MetaStore {
  data = $state<Meta | null>(null)
  loading = $state(false)

  async load(): Promise<void> {
    if (this.data || this.loading) return
    this.loading = true
    try {
      this.data = await metaApi.get()
    } catch {
      this.data = null
    } finally {
      this.loading = false
    }
  }
}

export const meta = new MetaStore()
