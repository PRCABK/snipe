# PowerShell packaging script for Snipe
param(
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

$cargoToml = Get-Content -LiteralPath (Join-Path $RepoRoot "Cargo.toml") -Raw
$workspaceVersionMatch = [regex]::Match(
    $cargoToml,
    '(?ms)^\[workspace\.package\].*?^version\s*=\s*"([^"]+)"'
)
if (!$workspaceVersionMatch.Success) {
    throw "Could not read workspace.package.version from Cargo.toml"
}
$workspaceVersion = $workspaceVersionMatch.Groups[1].Value
if ([string]::IsNullOrWhiteSpace($Version)) {
    $Version = $workspaceVersion
}
$Version = $Version.TrimStart('v')
if ($Version -ne $workspaceVersion) {
    throw "Release version '$Version' does not match Cargo workspace version '$workspaceVersion'"
}
if (!(Test-Path -LiteralPath (Join-Path $RepoRoot "Cargo.lock"))) {
    throw "Cargo.lock is required for a reproducible release build; generate and commit it before tagging"
}

Write-Host "==> Packaging Snipe v$Version..." -ForegroundColor Cyan

# Ensure dist output directory exists
$DistDir = Join-Path $RepoRoot "dist"
if (!(Test-Path $DistDir)) {
    New-Item -ItemType Directory -Path $DistDir | Out-Null
}

Write-Host "==> Building locked release workspace..." -ForegroundColor Cyan
cargo build --workspace --release --locked
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
}

$ExePath = Join-Path $RepoRoot "target\release\snipe.exe"
if (!(Test-Path -LiteralPath $ExePath)) {
    throw "Release binary was not produced at $ExePath"
}

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

if (!$IsccPath) {
    throw "ISCC.exe (Inno Setup 6) was not found"
}

Write-Host "==> Compiling installer with $IsccPath..." -ForegroundColor Cyan
& $IsccPath "/DMyAppVersion=$Version" (Join-Path $RepoRoot "installer\snipe.iss")
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup failed with exit code $LASTEXITCODE"
}

$SetupPath = Join-Path $DistDir "Snipe-Setup-$Version-x64.exe"
if (!(Test-Path -LiteralPath $SetupPath)) {
    throw "Expected installer was not produced at $SetupPath"
}

Write-Host "==> Packaging complete: $SetupPath" -ForegroundColor Green
