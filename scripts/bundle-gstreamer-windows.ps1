# Copies the GStreamer runtime next to StationScout.exe so the Windows build
# runs without a separate GStreamer install. The layout matches what
# rust/src/gst_runtime.rs looks for:
#   <dest>\*.dll                                    core and dependency DLLs
#   <dest>\lib\gstreamer-1.0\*.dll                  plugins
#   <dest>\lib\gio\modules\*.dll                    GIO TLS module (HTTPS)
#   <dest>\libexec\gstreamer-1.0\gst-plugin-scanner.exe
param(
    [Parameter(Mandatory = $true)][string]$GstRoot,
    [Parameter(Mandatory = $true)][string]$Dest
)

$ErrorActionPreference = "Stop"

function Copy-Dir([string]$from, [string]$to, [string]$filter, [bool]$required) {
    if (-not (Test-Path $from)) {
        if ($required) { throw "GStreamer folder not found: $from" }
        Write-Host "Skipping missing folder $from"
        return
    }
    New-Item -ItemType Directory -Force $to | Out-Null
    $files = Get-ChildItem -Path $from -Filter $filter -File
    if ($required -and $files.Count -eq 0) { throw "No $filter files in $from" }
    $files | Copy-Item -Destination $to -Force
    Write-Host "Copied $($files.Count) file(s) from $from"
}

Copy-Dir (Join-Path $GstRoot "bin") $Dest "*.dll" $true
Copy-Dir (Join-Path $GstRoot "lib\gstreamer-1.0") (Join-Path $Dest "lib\gstreamer-1.0") "*.dll" $true
Copy-Dir (Join-Path $GstRoot "lib\gio\modules") (Join-Path $Dest "lib\gio\modules") "*.dll" $false
Copy-Dir (Join-Path $GstRoot "libexec\gstreamer-1.0") (Join-Path $Dest "libexec\gstreamer-1.0") "gst-plugin-scanner.exe" $false

# The MSVC runtime, in case the user's PC doesn't have the VC++ redistributable.
# Microsoft allows app-local copies of these files.
foreach ($dll in "vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll") {
    $src = Join-Path $env:SystemRoot "System32\$dll"
    if ((Test-Path $src) -and -not (Test-Path (Join-Path $Dest $dll))) {
        Copy-Item $src $Dest
    }
}

$bytes = (Get-ChildItem -Path $Dest -Recurse -File | Measure-Object -Sum Length).Sum
Write-Host ("Bundle size: {0:N1} MB" -f ($bytes / 1MB))
