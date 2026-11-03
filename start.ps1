$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $projectRoot

Write-Host 'Membangun frontend...'
Push-Location (Join-Path $projectRoot 'frontend')
bun install --frozen-lockfile
bun run check
bun run build
Pop-Location

# Muat variabel dari .env (diabaikan Git). Kunci API tidak disimpan di skrip ini.
$envFile = Join-Path $projectRoot '.env'
if (Test-Path -LiteralPath $envFile) {
    foreach ($line in Get-Content -LiteralPath $envFile) {
        $trimmed = $line.Trim()
        if (-not $trimmed -or $trimmed.StartsWith('#')) { continue }
        $index = $trimmed.IndexOf('=')
        if ($index -le 0) { continue }
        $name = $trimmed.Substring(0, $index).Trim()
        $value = $trimmed.Substring($index + 1).Trim()
        if ($name.StartsWith('export ')) { $name = $name.Substring(7).Trim() }
        if ($value.Length -ge 2) {
            $first = $value.Substring(0, 1)
            if (($first -eq '"' -or $first -eq "'") -and $value.EndsWith($first)) {
                $value = $value.Substring(1, $value.Length - 2)
            }
        }
        [Environment]::SetEnvironmentVariable($name, $value, 'Process')
    }
} else {
    Write-Warning 'File .env tidak ditemukan. Salin .env.example menjadi .env lalu isi variabelnya.'
}

# Bawaan pengembangan bila tidak diatur di .env.
if (-not $env:LAPORAN_DATA_DIR) { $env:LAPORAN_DATA_DIR = Join-Path $projectRoot 'data' }
if (-not $env:LAPORAN_PORT) { $env:LAPORAN_PORT = '4317' }
if (-not $env:LAPORAN_CHROME) { $env:LAPORAN_CHROME = 'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe' }

Push-Location (Join-Path $projectRoot 'backend')
cargo run --release
Pop-Location
