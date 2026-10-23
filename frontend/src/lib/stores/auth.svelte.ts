import { authApi } from '../api'
import type { User } from '../types'

class AuthStore {
  user = $state<User | null>(null)
  loading = $state(true)
  error = $state('')

  async bootstrap(): Promise<void> {
    this.loading = true
    try {
      this.user = await authApi.me()
    } catch {
      this.user = null
    } finally {
      this.loading = false
    }
  }

  async login(username: string, password: string): Promise<void> {
    this.error = ''
    this.loading = true
    try {
      this.user = await authApi.login(username, password)
    } catch (error) {
      this.error = error instanceof Error ? error.message : 'Gagal masuk.'
      throw error
    } finally {
      this.loading = false
    }
  }

  async logout(): Promise<void> {
    try {
      await authApi.logout()
    } catch {
      // Sesi mungkin sudah kedaluwarsa.
    }
    this.user = null
  }

  clear(): void {
    this.user = null
  }
}

export const auth = new AuthStore()
