#Requires -Version 5
$ErrorActionPreference = 'Stop'

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host 'error: cargo not found' -ForegroundColor Red
    Write-Host 'Install Rust first: https://rustup.rs'
    exit 1
}

Push-Location $PSScriptRoot
try {
    Write-Host 'Building urun (release)...'
    cargo build --release
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $dest = Join-Path $HOME '.local\bin'
    if (-not (Test-Path $dest)) {
        New-Item -ItemType Directory -Path $dest -Force | Out-Null
    }

    $src = Join-Path $PSScriptRoot 'target\release\urun.exe'
    $out = Join-Path $dest 'urun.exe'
    Copy-Item -Path $src -Destination $out -Force
    Write-Host "Installed: $out"

    $destNorm = $dest.TrimEnd('\').ToLower()
    $userPath = [Environment]::GetEnvironmentVariable('PATH', 'User')
    $pathDirs = @()
    if ($userPath) { $pathDirs += $userPath -split ';' }
    $pathDirs += $env:PATH -split ';'
    $inPath = $pathDirs | Where-Object { $_ -and ($_.TrimEnd('\').ToLower() -eq $destNorm) }

    if (-not $inPath) {
        Write-Host ''
        Write-Warning "$dest is not in your PATH."
        Write-Host 'Add it permanently with this PowerShell command:'
        Write-Host ''
        Write-Host "    [Environment]::SetEnvironmentVariable('PATH', [Environment]::GetEnvironmentVariable('PATH','User') + ';$dest', 'User')" -ForegroundColor Cyan
        Write-Host ''
        Write-Host 'Then restart your shell.'
        Write-Host '(Or edit PATH via System Properties > Environment Variables.)'
    }
} finally {
    Pop-Location
}
