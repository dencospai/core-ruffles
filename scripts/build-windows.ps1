[CmdletBinding()]
param([switch]$DebugBuild)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    foreach ($tool in @('cargo', 'rustup', 'java')) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            throw "Missing $tool. See README.md for the Windows prerequisites."
        }
    }
    $target = 'x86_64-pc-windows-msvc'
    rustup target add $target
    if ($LASTEXITCODE -ne 0) { throw 'rustup target add failed' }
    $buildArgs = @('build', '--target', $target)
    if (Test-Path Cargo.lock) { $buildArgs += '--locked' }
    $profile = 'debug'
    if (-not $DebugBuild) { $buildArgs += '--release'; $profile = 'release' }
    cargo @buildArgs
    if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed' }
    $destination = 'dist/windows-x64'
    New-Item -ItemType Directory -Force $destination | Out-Null
    Copy-Item "target/$target/$profile/ruffle_libretro.dll" $destination
    Copy-Item 'ruffle_libretro.info', 'README.md', 'LICENSE-MIT', 'LICENSE-APACHE' $destination
    Write-Host "Core built: $destination/ruffle_libretro.dll"
} finally {
    Pop-Location
}

