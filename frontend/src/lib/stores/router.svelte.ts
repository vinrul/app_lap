export type RouteName = 'dashboard' | 'periods' | 'period' | 'imports' | 'users' | 'present'
export type PeriodTab = 'overview' | 'analysis' | 'records' | 'design' | 'documents'

export interface Route {
  name: RouteName
  id?: string
  tab?: PeriodTab
  path: string
}

const periodTabs: PeriodTab[] = ['overview', 'analysis', 'records', 'design', 'documents']

function parseRoute(): Route {
  const raw = window.location.hash.replace(/^#/, '') || '/dashboard'
  const path = raw.split('?')[0]
  const parts = path.split('/').filter(Boolean)

  if (parts[0] === 'periods' && parts[1]) {
    const tab = periodTabs.includes(parts[2] as PeriodTab) ? (parts[2] as PeriodTab) : 'overview'
    return { name: 'period', id: parts[1], tab, path }
  }
  if (parts[0] === 'periods') return { name: 'periods', path }
  if (parts[0] === 'present' && parts[1]) return { name: 'present', id: parts[1], path }
  if (parts[0] === 'imports') return { name: 'imports', path }
  if (parts[0] === 'users') return { name: 'users', path }
  return { name: 'dashboard', path: '/dashboard' }
}

class RouterStore {
  route = $state<Route>(parseRoute())

  sync = (): void => {
    this.route = parseRoute()
    window.scrollTo({ top: 0, behavior: 'instant' })
  }
}

export const router = new RouterStore()

export function navigate(path: string): void {
  const target = path.startsWith('/') ? path : `/${path}`
  if (window.location.hash === `#${target}`) {
    router.sync()
    return
  }
  window.location.hash = target
}
