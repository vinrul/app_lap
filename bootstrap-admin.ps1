$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$admin = Read-Host 'Nama pengguna admin pertama'
$secure = Read-Host 'Kata sandi admin (minimal 12 karakter)' -AsSecureString
$credential = [System.Net.NetworkCredential]::new('', $secure)
$env:LAPORAN_ADMIN_USER = $admin
$env:LAPORAN_ADMIN_PASSWORD = $credential.Password
$env:LAPORAN_DATA_DIR = Join-Path $projectRoot 'data'
Push-Location (Join-Path $projectRoot 'backend')
cargo run --release -- bootstrap-admin
Pop-Location
Remove-Item Env:\LAPORAN_ADMIN_PASSWORD -ErrorAction SilentlyContinue
Remove-Item Env:\LAPORAN_ADMIN_USER -ErrorAction SilentlyContinue
