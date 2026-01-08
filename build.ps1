# Build script for sts-rust
# Usage: .\build.ps1

$ErrorActionPreference = "Stop"

# Generate build number (resets daily)
$buildNumberFile = ".build_number"
$currentDate = (Get-Date).ToUniversalTime().AddHours(8).ToString("yyyyMMdd")

if (Test-Path $buildNumberFile) {
    $content = (Get-Content $buildNumberFile -Raw).Trim()
    $parts = $content -split '_'
    if ($parts.Length -eq 2) {
        $storedDate = $parts[0]
        $buildNum = [int]$parts[1]

        if ($storedDate -eq $currentDate) {
            $buildNum = $buildNum + 1
        } else {
            $buildNum = 1
        }
    } else {
        $buildNum = 1
    }
} else {
    $buildNum = 1
}

$version = "${currentDate}_${buildNum}"
Set-Content $buildNumberFile $version -NoNewline

Write-Host "Build version: $version" -ForegroundColor Cyan
Write-Host "Building release..." -ForegroundColor Cyan
cargo build --release

if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

$sourceExe = "target\release\sts.exe"
$outputExe = "sts3.0_build$version.exe"

# Check if source file exists
if (-not (Test-Path $sourceExe)) {
    Write-Host "Error: $sourceExe not found!" -ForegroundColor Red
    exit 1
}

# Copy to output name first
Copy-Item $sourceExe $outputExe -Force

# Compress with UPX
Write-Host "Compressing with UPX..." -ForegroundColor Cyan
$upxPath = "$PSScriptRoot\upx-4.2.4-win64\upx.exe"
& $upxPath --best --lzma $outputExe

if ($LASTEXITCODE -ne 0) {
    Write-Host "UPX compression failed!" -ForegroundColor Red
    exit 1
}

# Show result
$fileInfo = Get-Item $outputExe
$sizeKB = [math]::Round($fileInfo.Length / 1024, 2)
Write-Host ""
Write-Host "Build complete: $outputExe ($sizeKB KB)" -ForegroundColor Green
