# Baut alle Spiele + Launcher im Release-Modus und packt die Exes
# in einen verschiebbaren Ordner "Doodle Box/".
# Ausführen:  powershell -ExecutionPolicy Bypass -File scripts\bundle.ps1

$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

cargo build --release
if ($LASTEXITCODE -ne 0) { throw "Build fehlgeschlagen." }

$bundle = Join-Path (Get-Location) "Doodle Box"
New-Item -ItemType Directory -Force -Path $bundle | Out-Null

# Launcher unter verständlichem Namen, Spiele daneben (gleiche Ordner!)
Copy-Item "target\release\doodle-launcher.exe" (Join-Path $bundle "Doodle Box.exe") -Force
Get-ChildItem "target\release\doodle-*.exe" |
    Where-Object { $_.Name -ne "doodle-launcher.exe" } |
    ForEach-Object { Copy-Item $_.FullName $bundle -Force }

Write-Host ""
Write-Host "Fertig! Startbar über:  $bundle\Doodle Box.exe"
Get-ChildItem $bundle | Select-Object Name, Length
