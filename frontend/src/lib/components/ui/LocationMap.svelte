<script lang="ts">
  import { onMount } from 'svelte'
  import 'leaflet/dist/leaflet.css'
  import type { DivIcon, Map as LeafletMap, Marker } from 'leaflet'
  import { ExternalLink, Navigation } from 'lucide-svelte'

  type Props = {
    latitude: number
    longitude: number
    label?: string
    detail?: string
  }

  let { latitude, longitude, label = 'Lokasi kegiatan', detail = '' }: Props = $props()

  let containerEl = $state<HTMLDivElement | null>(null)
  let map: LeafletMap | null = null
  let marker: Marker | null = null
  let leaflet: typeof import('leaflet') | null = null
  let loadError = $state(false)

  const lat = $derived(Number(latitude))
  const lng = $derived(Number(longitude))
  const valid = $derived(
    Number.isFinite(lat) &&
      Number.isFinite(lng) &&
      lat >= -90 &&
      lat <= 90 &&
      lng >= -180 &&
      lng <= 180,
  )
  const coordinateLabel = $derived(
    valid ? `${lat.toFixed(6)}, ${lng.toFixed(6)}` : 'Koordinat tidak valid',
  )
  const googleMapsUrl = $derived(
    valid
      ? `https://www.google.com/maps/search/?api=1&query=${encodeURIComponent(`${lat},${lng}`)}`
      : '',
  )
  const directionsUrl = $derived(
    valid
      ? `https://www.google.com/maps/dir/?api=1&destination=${encodeURIComponent(`${lat},${lng}`)}`
      : '',
  )

  onMount(() => {
    let cancelled = false
    let observer: ResizeObserver | null = null
    let timer = 0

    void (async () => {
      const container = containerEl
      if (!container || !valid) return
      try {
        const module = await import('leaflet')
        if (cancelled) return
        const L = (module.default ?? module) as unknown as typeof import('leaflet')
        leaflet = L
        map = L.map(container, {
          center: [lat, lng],
          zoom: 16,
          scrollWheelZoom: false,
          zoomControl: true,
        })

        L.tileLayer('/api/map/tiles/{z}/{x}/{y}', {
          maxZoom: 19,
          attribution:
            '&copy; <a href="https://www.openstreetmap.org/copyright" target="_blank" rel="noopener">OpenStreetMap</a> contributors',
        }).addTo(map)

        updateMarker(lat, lng, label, detail)
        observer = new ResizeObserver(() => map?.invalidateSize())
        observer.observe(container)
        timer = window.setTimeout(() => map?.invalidateSize(), 180)
      } catch {
        if (!cancelled) loadError = true
      }
    })()

    return () => {
      cancelled = true
      window.clearTimeout(timer)
      observer?.disconnect()
      map?.remove()
      map = null
      marker = null
      leaflet = null
    }
  })

  $effect(() => {
    const nextLat = lat
    const nextLng = lng
    const nextLabel = label
    const nextDetail = detail
    if (!map || !valid) return
    updateMarker(nextLat, nextLng, nextLabel, nextDetail)
  })

  function markerIcon(L: typeof import('leaflet')): DivIcon {
    return L.divIcon({
      className: 'location-map-marker',
      html: '<span style="display:flex;width:22px;height:22px;border-radius:999px;background:#143d30;border:3px solid #ffffff;box-shadow:0 3px 10px rgba(20,61,48,.35);box-sizing:border-box;"></span>',
      iconSize: [22, 22],
      iconAnchor: [11, 11],
      popupAnchor: [0, -12],
    })
  }

  function popupContent(title: string, subtitle: string): HTMLElement {
    const wrapper = document.createElement('div')
    wrapper.style.minWidth = '160px'
    const heading = document.createElement('strong')
    heading.style.display = 'block'
    heading.style.fontSize = '12px'
    heading.style.color = '#143d30'
    heading.textContent = title || 'Lokasi kegiatan'
    wrapper.appendChild(heading)
    if (subtitle) {
      const body = document.createElement('span')
      body.style.display = 'block'
      body.style.marginTop = '2px'
      body.style.fontSize = '10px'
      body.style.color = '#6d7872'
      body.textContent = subtitle
      wrapper.appendChild(body)
    }
    return wrapper
  }

  function updateMarker(nextLat: number, nextLng: number, nextLabel: string, nextDetail: string): void {
    const L = leaflet
    if (!map || !L) return
    const point = L.latLng(nextLat, nextLng)
    map.setView(point, map.getZoom() || 16)
    if (!marker) {
      marker = L.marker(point, { icon: markerIcon(L), title: nextLabel }).addTo(map)
    } else {
      marker.setLatLng(point)
      marker.options.title = nextLabel
    }
    marker.bindPopup(popupContent(nextLabel, nextDetail), { closeButton: false, offset: [0, -2] })
  }
</script>

<div class="overflow-hidden rounded-xl border border-line bg-white">
  {#if valid}
    <div
      class="relative isolate z-0 h-64 w-full bg-stone-100 sm:h-72"
      bind:this={containerEl}
      role="region"
      aria-label={`Peta lokasi ${label || 'kegiatan'}`}
    >
      {#if loadError}
        <div class="absolute inset-0 z-[1001] flex items-center justify-center bg-stone-100 px-4 text-center text-xs text-ink-muted">
          Peta tidak dapat dimuat. Periksa koneksi internet lalu buka kembali detail ini.
        </div>
      {/if}
    </div>
    <div class="flex flex-wrap items-center justify-between gap-2 border-t border-line bg-stone-50/70 px-3 py-2.5">
      <div class="min-w-0">
        <p class="truncate text-[11px] font-semibold text-ink">{label || 'Lokasi kegiatan'}</p>
        <p class="mt-0.5 font-mono text-[10px] text-ink-muted">{coordinateLabel}</p>
      </div>
      <div class="flex shrink-0 flex-wrap items-center gap-1.5">
        <a class="btn btn-secondary btn-sm" href={googleMapsUrl} target="_blank" rel="noopener">
          <ExternalLink size={13} /> Google Maps
        </a>
        <a class="btn btn-ghost btn-sm" href={directionsUrl} target="_blank" rel="noopener">
          <Navigation size={13} /> Rute
        </a>
      </div>
    </div>
  {:else}
    <div class="flex items-center justify-center px-4 py-10 text-center text-xs text-ink-muted">
      Koordinat lokasi tidak valid.
    </div>
  {/if}
</div>
