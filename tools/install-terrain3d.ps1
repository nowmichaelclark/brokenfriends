# Installs the pinned version of the Terrain3D addon into game\addons\.
# Run this one time after you clone the repo:
#   powershell -ExecutionPolicy Bypass -File tools\install-terrain3d.ps1
$ErrorActionPreference = "Stop"

$Version = "v1.1.0-stable"
$Sha256 = "51b78bc758f5759cbc8eaaab14e5cf4e8661717e265fdd760f5abf945f15b243"
$Url = "https://github.com/TokisanGames/Terrain3D/releases/download/$Version/Terrain3D_$Version.zip"

$Root = Split-Path -Parent $PSScriptRoot
$Dest = Join-Path $Root "game\addons\terrain_3d"
$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("terrain3d-" + [System.Guid]::NewGuid())
New-Item -ItemType Directory -Path $Tmp | Out-Null

try {
    Write-Host "Downloading Terrain3D $Version ..."
    $Zip = Join-Path $Tmp "terrain3d.zip"
    Invoke-WebRequest -Uri $Url -OutFile $Zip
    $Hash = (Get-FileHash -Algorithm SHA256 $Zip).Hash.ToLower()
    if ($Hash -ne $Sha256) {
        throw "The checksum of the download is wrong. Nothing was installed."
    }
    Expand-Archive -Path $Zip -DestinationPath $Tmp
    if (Test-Path $Dest) { Remove-Item -Recurse -Force $Dest }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Dest) | Out-Null
    Move-Item (Join-Path $Tmp "addons\terrain_3d") $Dest
    Set-Content -Path (Join-Path $Dest "BF_PINNED_VERSION") -Value $Version
    Write-Host "Terrain3D $Version is installed in game\addons\terrain_3d."
}
finally {
    Remove-Item -Recurse -Force $Tmp
}
