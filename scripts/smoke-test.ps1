# Smoke test script for Snipe installer and binary
param(
    [string]$SetupExePath = ""
)

$ErrorActionPreference = "Stop"
Write-Host "==> Starting Snipe Smoke Tests..." -ForegroundColor Cyan

$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

# Test single instance argument
$ExePath = Join-Path $RepoRoot "target\release\snipe.exe"
if (Test-Path $ExePath) {
    Write-Host "==> Testing command line flag --shutdown-for-update..." -ForegroundColor Yellow
    & $ExePath --shutdown-for-update
    Write-Host "==> Binary responded cleanly." -ForegroundColor Green
} else {
    Write-Host "==> $ExePath not found. Please build release first." -ForegroundColor Yellow
}

Write-Host "==> Smoke test finished." -ForegroundColor Green
