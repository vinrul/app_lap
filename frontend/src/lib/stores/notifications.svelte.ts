import { notificationsApi } from '../api'
import type { NotificationItem } from '../types'

class NotificationStore {
  items = $state<NotificationItem[]>([])
  unread = $state(0)
  loading = $state(false)

  async load(): Promise<void> {
    this.loading = true
    try {
      const data = await notificationsApi.list()
      this.items = data.items
      this.unread = data.unread
    } catch {
      // Badge tetap memakai nilai terakhir bila permintaan gagal.
    } finally {
      this.loading = false
    }
  }

  push(item: NotificationItem): void {
    if (this.items.some((existing) => existing.id === item.id)) return
    this.items = [item, ...this.items].slice(0, 50)
    this.unread += 1
  }

  setUnread(value: number): void {
    this.unread = value
  }

  async markAll(): Promise<void> {
    try {
      const data = await notificationsApi.markRead()
      this.unread = data.unread
      const now = new Date().toISOString()
      this.items = this.items.map((item) => (item.read_at ? item : { ...item, read_at: now }))
    } catch {
      // Abaikan; pengguna dapat mencoba lagi.
    }
  }

  async markRead(id: string): Promise<void> {
    const item = this.items.find((entry) => entry.id === id)
    if (item?.read_at) return
    try {
      const data = await notificationsApi.markRead([id])
      this.unread = data.unread
      const now = new Date().toISOString()
      this.items = this.items.map((entry) =>
        entry.id === id ? { ...entry, read_at: now } : entry,
      )
    } catch {
      // Abaikan.
    }
  }

  reset(): void {
    this.items = []
    this.unread = 0
  }
}

export const notifications = new NotificationStore()
