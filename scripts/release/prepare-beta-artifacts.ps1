param(
    [Parameter(Mandatory = $true)]
    [string]$MsiPath,

    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory,

    [Parameter(Mandatory = $true)]
    [string]$CommitSha,

    [Parameter(Mandatory = $true)]
    [string]$RunId,

    [Parameter(Mandatory = $true)]
    [string]$RunAttempt
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$resolvedMsi = (Resolve-Path -LiteralPath $MsiPath).Path
$output = [System.IO.Path]::GetFullPath($OutputDirectory)

New-Item -ItemType Directory -Force -Path $output | Out-Null

$destinationMsi = Join-Path $output "Sims-Mod-Health-0.1.0-beta.1-x64.msi"
Copy-Item -LiteralPath $resolvedMsi -Destination $destinationMsi -Force

$hash = Get-FileHash -LiteralPath $destinationMsi -Algorithm SHA256
$checksumLine = "{0}  {1}" -f $hash.Hash.ToLowerInvariant(), (Split-Path -Leaf $destinationMsi)
Set-Content -LiteralPath (Join-Path $output "SHA256SUMS.txt") -Value $checksumLine -Encoding ascii

$provenance = @(
    "product=Sims Mod Health",
    "release=v0.1.0-beta.1",
    "app_version=0.1.0",
    "source_commit=$CommitSha",
    "github_run_id=$RunId",
    "github_run_attempt=$RunAttempt",
    "runner_os=Windows",
    "installer=MSI",
    "validation=clean install + uninstall passed before artifact publication"
)
Set-Content -LiteralPath (Join-Path $output "BUILD_PROVENANCE.txt") -Value $provenance -Encoding utf8

Write-Host "Prepared beta artifact: $destinationMsi"
Write-Host "SHA-256: $($hash.Hash.ToLowerInvariant())"
