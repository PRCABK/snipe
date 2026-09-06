# PowerShell packaging script for Snipe
param(
    [string]$Version = "0.1.0"
)

$ErrorActionPreference = "Stop"
Write-Host "==> Packaging Snipe v$Version..." -ForegroundColor Cyan

$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

# Ensure dist output directory exists
$DistDir = Join-Path $RepoRoot "dist"
if (!(Test-Path $DistDir)) {
    New-Item -ItemType Directory -Path $DistDir | Out-Null
}

# 1. Check release executable
$ExePath = Join-Path $RepoRoot "target\release\snipe.exe"
if (!(Test-Path $ExePath)) {
    Write-Host "==> Release binary not found, building..." -ForegroundColor Yellow
    cargo build --release --workspace
}

if (!(Test-Path $ExePath)) {
    Write-Error "Failed to locate $ExePath after build!"
    exit 1
}

Write-Host "==> Found release binary: $ExePath" -ForegroundColor Green

# 2. Package portable zip
$ZipPath = Join-Path $DistDir "Snipe-v$Version-windows-x64-portable.zip"
Write-Host "==> Creating portable archive: $ZipPath"
Compress-Archive -Path $ExePath -DestinationPath $ZipPath -Force

# 3. Compile Inno Setup installer
$IsccCandidates = @(
    "ISCC.exe",
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe",
    "${env:LOCALAPPDATA}\Programs\Inno Setup 6\ISCC.exe"
)

$IsccPath = $null
foreach ($c in $IsccCandidates) {
    if (Get-Command $c -ErrorAction SilentlyContinue) {
        $IsccPath = $c
        break
    }
    if (Test-Path $c) {
        $IsccPath = $c
        break
    }
}

if ($IsccPath) {
    Write-Host "==> Compiling installer with $IsccPath..." -ForegroundColor Cyan
    & $IsccPath "/DMyAppVersion=$Version" (Join-Path $RepoRoot "installer\snipe.iss")
    Write-Host "==> Inno Setup installer generated in $DistDir" -ForegroundColor Green
} else {
    Write-Host "[WARNING] ISCC.exe (Inno Setup) not found. Portable zip was generated, but setup.exe was skipped." -ForegroundColor Yellow
}

Write-Host "==> Packaging complete!" -ForegroundColor Green
