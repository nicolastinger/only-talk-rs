# Installs the pre-push hook from scripts/pre-push into .git/hooks/pre-push

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$src = Join-Path $PSScriptRoot "pre-push"
$destDir = Join-Path $repoRoot ".git\hooks"
$dest = Join-Path $destDir "pre-push"

if (-not (Test-Path $destDir)) {
    Write-Error ".git\hooks not found - is this a git repository?"
}

Copy-Item -LiteralPath $src -Destination $dest -Force
Write-Host "Installed pre-push hook: $dest"