#Requires -Version 5
$ErrorActionPreference = 'Stop'

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host 'error: cargo not found' -ForegroundColor Red
    Write-Host 'Install Rust first: https://rustup.rs'
    exit 1
}

$targets = @(
    'x86_64-pc-windows-msvc',
    'aarch64-pc-windows-msvc'
)

Push-Location $PSScriptRoot
try {
    $installed = (& rustup target list --installed) -split "`n" | ForEach-Object { $_.Trim() }

    foreach ($t in $targets) {
        if ($installed -notcontains $t) {
            Write-Host "Installing Rust target: $t"
            rustup target add $t
            if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        }
    }

    $distDir = Join-Path $PSScriptRoot 'dist'
    if (-not (Test-Path $distDir)) {
        New-Item -ItemType Directory -Path $distDir -Force | Out-Null
    }

    foreach ($t in $targets) {
        Write-Host ''
        Write-Host "Building urun for $t (release)..." -ForegroundColor Cyan
        cargo build --release --target $t
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

        $src = Join-Path $PSScriptRoot "target\$t\release\urun.exe"
        $out = Join-Path $distDir "urun-$t.exe"
        Copy-Item -Path $src -Destination $out -Force
        Write-Host "  -> $out"
    }

    Write-Host ''
    Write-Host 'Done. Artifacts:' -ForegroundColor Green
    Get-ChildItem $distDir -Filter 'urun-*-pc-windows-msvc.exe' |
        ForEach-Object { Write-Host "  $($_.FullName)" }
} finally {
    Pop-Location
}
