<#
.SYNOPSIS
    Release helper for yImage.

.DESCRIPTION
    Bumps the version in Cargo.toml and installer/yImage.iss, refreshes
    Cargo.lock, lets you edit CHANGELOG.md, then commits and tags the
    release. Pushing the tag triggers .github/workflows/release.yml,
    which builds the Windows installer + portable zip on GitHub Actions.

.PARAMETER Version
    Semantic version string (X.Y.Z), without the leading "v".

.EXAMPLE
    .\scripts\release.ps1 0.2.0
#>

param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$Version
)

$ErrorActionPreference = "Stop"

# Jump to repo root so relative paths work no matter where the script is invoked.
$repoRoot = git rev-parse --show-toplevel
Set-Location $repoRoot

$status = git status --porcelain
if ($status) {
    Write-Host $status
    throw "Working tree has uncommitted changes. Commit or stash first."
}

if (git tag -l "v$Version") {
    throw "Tag v$Version already exists."
}

Write-Host "==> Bumping yImage to v$Version"

# Cargo.toml: replace the first `version = "..."` line inside [package].
$cargo = Get-Content Cargo.toml -Raw
$pattern = '(?ms)(\[package\].*?\n)version\s*=\s*"[^"]+"'
$cargo = [regex]::Replace($cargo, $pattern, {
    param($m)
    "$($m.Groups[1].Value)version = `"$Version`""
}, 'None')
Set-Content -NoNewline -Encoding UTF8 Cargo.toml $cargo

# Inno Setup AppVersion default.
$iss = Get-Content installer/yImage.iss -Raw
$iss = [regex]::Replace($iss, '#define AppVersion "[^"]+"', "#define AppVersion `"$Version`"")
Set-Content -NoNewline -Encoding UTF8 installer/yImage.iss $iss

# Refresh Cargo.lock.
$updateFailed = $false
try { cargo update -p yimage --offline 2>$null } catch { $updateFailed = $true }
if ($LASTEXITCODE -ne 0 -or $updateFailed) {
    cargo update -p yimage
}

Write-Host "==> Updated Cargo.toml, installer/yImage.iss, Cargo.lock"
Write-Host ""
Write-Host "==> Open CHANGELOG.md and fill in the [$Version] section, then come back."
Read-Host "Press Enter once CHANGELOG.md is ready (or Ctrl+C to abort)" | Out-Null

git add Cargo.toml Cargo.lock installer/yImage.iss CHANGELOG.md
git commit -m "chore: release v$Version"
git tag -a "v$Version" -m "v$Version"

$branch = git rev-parse --abbrev-ref HEAD

Write-Host ""
Write-Host "==> Tagged v$Version on branch $branch"
Write-Host ""
Write-Host "To publish (runs the GitHub Actions release workflow):"
Write-Host "    git push origin $branch"
Write-Host "    git push origin v$Version"
Write-Host ""
Write-Host "Or push both at once:"
Write-Host "    git push --atomic origin $branch v$Version"
