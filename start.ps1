$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $projectRoot

Write-Host 'Membangun frontend...'
Push-Location (Join-Path $projectRoot 'frontend')
bun install --frozen-lockfile
bun run check
bun run build
Pop-Location

$env:LAPORAN_DATA_DIR = Join-Path $projectRoot 'data'
$env:LAPORAN_PORT = '4317'
$env:LAPORAN_CHROME = 'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe'

Push-Location (Join-Path $projectRoot 'backend')
cargo run --release
Pop-Location
