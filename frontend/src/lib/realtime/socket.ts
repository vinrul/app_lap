import type { RealtimeClientMessage, RealtimeServerMessage } from '../types'

export type RealtimeStatus = 'off' | 'connecting' | 'online'

type MessageHandler = (message: RealtimeServerMessage) => void
type StatusHandler = (status: RealtimeStatus) => void

const RECONNECT_DELAYS = [500, 1_000, 2_000, 5_000, 10_000]
const HEARTBEAT_MS = 20_000
const MAX_QUEUE = 20

/**
 * Satu koneksi WebSocket untuk seluruh aplikasi.
 *
 * Menyambung ulang otomatis dengan backoff, mengirim heartbeat, dan menyimpan
 * pesan singkat selama koneksi belum terbuka. Sumber kebenaran tetap HTTP:
 * setelah tersambung, pemanggil mengirim `subscribe` dan server membalas
 * snapshot terbaru.
 */
class RealtimeSocket {
  private socket: WebSocket | null = null
  private messageHandlers = new Set<MessageHandler>()
  private statusHandlers = new Set<StatusHandler>()
  private queue: RealtimeClientMessage[] = []
  private heartbeat: ReturnType<typeof setInterval> | null = null
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null
  private attempts = 0
  private status: RealtimeStatus = 'off'
  private manualClose = false
  private listenersAttached = false
  private sessionValue = ''

  /** Penanda sesi dari pesan `hello`, untuk mencocokkan `session_revoked`. */
  get sessionId(): string {
    return this.sessionValue
  }

  get currentStatus(): RealtimeStatus {
    return this.status
  }

  get isOnline(): boolean {
    return this.socket?.readyState === WebSocket.OPEN
  }

  connect(): void {
    if (typeof WebSocket === 'undefined') return
    this.attachListeners()
    this.manualClose = false
    if (
      this.socket &&
      (this.socket.readyState === WebSocket.OPEN ||
        this.socket.readyState === WebSocket.CONNECTING)
    ) {
      return
    }
    this.clearReconnect()
    const scheme = window.location.protocol === 'https:' ? 'wss' : 'ws'
    let socket: WebSocket
    try {
      socket = new WebSocket(`${scheme}://${window.location.host}/api/ws`)
    } catch {
      this.scheduleReconnect()
      return
    }
    this.socket = socket
    this.setStatus('connecting')

    socket.onopen = () => {
      if (this.socket !== socket) return
      this.attempts = 0
      this.setStatus('online')
      this.startHeartbeat()
      const pending = this.queue
      this.queue = []
      for (const message of pending) this.write(message)
    }

    socket.onmessage = (event) => {
      if (this.socket !== socket) return
      let message: RealtimeServerMessage
      try {
        message = JSON.parse(String(event.data)) as RealtimeServerMessage
      } catch {
        return
      }
      if (message.type === 'pong') return
      if (message.type === 'hello') this.sessionValue = message.session
      for (const handler of this.messageHandlers) handler(message)
    }

    socket.onclose = () => {
      if (this.socket !== socket) return
      this.socket = null
      this.stopHeartbeat()
      this.setStatus('off')
      this.scheduleReconnect()
    }

    socket.onerror = () => {
      // `onclose` selalu menyusul; penanganan cukup di sana.
    }
  }

  disconnect(): void {
    this.manualClose = true
    this.clearReconnect()
    this.stopHeartbeat()
    this.queue = []
    const socket = this.socket
    this.socket = null
    this.setStatus('off')
    socket?.close()
  }

  /** Mengirim pesan. Saat offline pesan kecil disimpan untuk dikirim ulang. */
  send(message: RealtimeClientMessage): boolean {
    if (this.isOnline) {
      this.write(message)
      return true
    }
    this.queue.push(message)
    if (this.queue.length > MAX_QUEUE) this.queue.shift()
    return false
  }

  onMessage(handler: MessageHandler): () => void {
    this.messageHandlers.add(handler)
    return () => this.messageHandlers.delete(handler)
  }

  onStatus(handler: StatusHandler): () => void {
    this.statusHandlers.add(handler)
    handler(this.status)
    return () => this.statusHandlers.delete(handler)
  }

  private write(message: RealtimeClientMessage): void {
    if (!this.isOnline) {
      this.queue.push(message)
      return
    }
    this.socket?.send(JSON.stringify(message))
  }

  private startHeartbeat(): void {
    this.stopHeartbeat()
    this.heartbeat = setInterval(() => {
      if (this.isOnline) this.socket?.send(JSON.stringify({ type: 'ping' }))
    }, HEARTBEAT_MS)
  }

  private stopHeartbeat(): void {
    if (this.heartbeat !== null) {
      clearInterval(this.heartbeat)
      this.heartbeat = null
    }
  }

  private scheduleReconnect(): void {
    if (this.manualClose || this.reconnectTimer !== null) return
    const delay = RECONNECT_DELAYS[Math.min(this.attempts, RECONNECT_DELAYS.length - 1)]
    this.attempts += 1
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null
      this.connect()
    }, delay)
  }

  private clearReconnect(): void {
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
  }

  private setStatus(status: RealtimeStatus): void {
    if (this.status === status) return
    this.status = status
    for (const handler of this.statusHandlers) handler(status)
  }

  /** Menyambung ulang saat tab kembali aktif atau jaringan pulih. */
  private attachListeners(): void {
    if (this.listenersAttached || typeof window === 'undefined') return
    this.listenersAttached = true
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState !== 'visible') return
      if (!this.isOnline) {
        this.attempts = 0
        this.connect()
      }
    })
    window.addEventListener('online', () => {
      this.attempts = 0
      this.connect()
    })
  }
}

export const realtime = new RealtimeSocket()
