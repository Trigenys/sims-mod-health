param(
    [Parameter(Mandatory = $true)]
    [string]$MsiPath
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$resolvedMsi = (Resolve-Path -LiteralPath $MsiPath).Path
$productName = "Sims Mod Health"

function Get-SimsModHealthUninstallEntries {
    $paths = @(
        "Registry::HKEY_LOCAL_MACHINE\Software\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "Registry::HKEY_LOCAL_MACHINE\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
        "Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Uninstall\*"
    )

    @(
        foreach ($path in $paths) {
            Get-ItemProperty -Path $path -ErrorAction SilentlyContinue |
                Where-Object {
                    $_.PSObject.Properties.Name -contains "DisplayName" -and
                    $_.DisplayName -eq $productName
                }
        }
    )
}

function Invoke-Msi {
    param(
        [Parameter(Mandatory = $true)]
        [ValidateSet("install", "uninstall")]
        [string]$Operation
    )

    $verb = if ($Operation -eq "install") { "/i" } else { "/x" }
    $arguments = @(
        $verb,
        ('"{0}"' -f $resolvedMsi),
        "/qn",
        "/norestart"
    )

    $process = Start-Process -FilePath "msiexec.exe" -ArgumentList $arguments -Wait -PassThru

    if ($process.ExitCode -notin @(0, 3010)) {
        throw "MSI $Operation failed with exit code $($process.ExitCode)."
    }

    if ($process.ExitCode -eq 3010) {
        Write-Host "MSI $Operation requested a reboot; treating 3010 as successful installation state."
    }
}

$before = @(Get-SimsModHealthUninstallEntries)
if ($before.Count -ne 0) {
    throw "Clean-runner precondition failed: Sims Mod Health is already registered as installed."
}

Write-Host "Installing $resolvedMsi on clean Windows runner..."
Invoke-Msi -Operation install
Start-Sleep -Seconds 2

$installed = @(Get-SimsModHealthUninstallEntries)
if ($installed.Count -eq 0) {
    throw "Installer returned success but no Sims Mod Health uninstall registration was found."
}

$entry = $installed | Select-Object -First 1
Write-Host "Installed product registration found: $($entry.DisplayName) $($entry.DisplayVersion)"

$installLocationProperty = $entry.PSObject.Properties["InstallLocation"]
if (
    $null -ne $installLocationProperty -and
    -not [string]::IsNullOrWhiteSpace([string]$installLocationProperty.Value) -and
    -not (Test-Path -LiteralPath ([string]$installLocationProperty.Value))
) {
    throw "Registered InstallLocation does not exist: $($installLocationProperty.Value)"
}

Write-Host "Uninstalling the same MSI..."
Invoke-Msi -Operation uninstall
Start-Sleep -Seconds 2

$after = @(Get-SimsModHealthUninstallEntries)
if ($after.Count -ne 0) {
    throw "Uninstall returned success but Sims Mod Health is still registered as installed."
}

Write-Host "Clean Windows install/uninstall validation passed."
