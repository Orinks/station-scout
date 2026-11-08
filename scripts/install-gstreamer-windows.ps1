param(
    [string]$Version = "1.28.3",
    [string]$InstallDir = (Join-Path $PSScriptRoot "..\.deps\gstreamer"),
    [string]$Sha256 = "f01b4fb30b3aefdcdf4251d4a7464fb68197d15a763a2a94b667b93f88cc6216"
)

$ErrorActionPreference = "Stop"

$installDirPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($InstallDir)
$gstBin = Join-Path $installDirPath "bin"
$pkgConfigPath = Join-Path $installDirPath "lib\pkgconfig"

if (-not (Test-Path (Join-Path $gstBin "gstreamer-1.0-0.dll")) -or
    -not (Test-Path (Join-Path $pkgConfigPath "gstreamer-1.0.pc"))) {
    $downloadDir = if ($env:RUNNER_TEMP) {
        $env:RUNNER_TEMP
    } else {
        Join-Path $PSScriptRoot "..\.deps\downloads"
    }
    New-Item -ItemType Directory -Force $downloadDir | Out-Null

    $installer = Join-Path $downloadDir "gstreamer-1.0-msvc-x86_64-$Version.exe"
    $url = "https://gstreamer.freedesktop.org/data/pkg/windows/$Version/msvc/gstreamer-1.0-msvc-x86_64-$Version.exe"

    if (-not (Test-Path $installer)) {
        # The GStreamer mirror can be slow or drop large downloads. curl
        # resumes partial downloads and gives up on a stalled connection
        # instead of hanging until the job times out.
        $partial = "$installer.part"
        for ($attempt = 1; $attempt -le 5; $attempt++) {
            & curl.exe -L --fail --silent --show-error --connect-timeout 30 `
                --speed-limit 20000 --speed-time 60 -C - -o $partial $url
            if ($LASTEXITCODE -eq 0) { break }
            if ($attempt -eq 5) { throw "GStreamer download failed after $attempt attempts (curl exit $LASTEXITCODE)." }
            Write-Host "GStreamer download attempt $attempt failed (curl exit $LASTEXITCODE). Retrying."
            Start-Sleep -Seconds (5 * $attempt)
        }
        Move-Item $partial $installer -Force
    }

    $actualSha256 = (Get-FileHash -Algorithm SHA256 $installer).Hash.ToLowerInvariant()
    if ($actualSha256 -ne $Sha256.ToLowerInvariant()) {
        throw "GStreamer installer checksum mismatch. Expected $Sha256, got $actualSha256."
    }

    New-Item -ItemType Directory -Force $installDirPath | Out-Null
    $args = "/VERYSILENT /NORESTART /CURRENTUSER /TYPE=devel /DIR=`"$installDirPath`""
    $process = Start-Process -FilePath $installer -ArgumentList $args -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "GStreamer installer failed with exit code $($process.ExitCode)."
    }
}

if (-not (Test-Path (Join-Path $gstBin "pkg-config.exe"))) {
    throw "GStreamer pkg-config.exe was not found under $gstBin."
}

if ($env:GITHUB_PATH) {
    Add-Content -Path $env:GITHUB_PATH -Value $gstBin
}
if ($env:GITHUB_ENV) {
    Add-Content -Path $env:GITHUB_ENV -Value "PKG_CONFIG_PATH=$pkgConfigPath"
    Add-Content -Path $env:GITHUB_ENV -Value "GSTREAMER_1_0_ROOT_MSVC_X86_64=$installDirPath"
    Add-Content -Path $env:GITHUB_ENV -Value "GSTREAMER_ROOT_X86_64=$installDirPath"
}

$env:PATH = "$gstBin;$env:PATH"
$env:PKG_CONFIG_PATH = $pkgConfigPath
$env:GSTREAMER_1_0_ROOT_MSVC_X86_64 = $installDirPath
$env:GSTREAMER_ROOT_X86_64 = $installDirPath

& (Join-Path $gstBin "gst-launch-1.0.exe") --version
& (Join-Path $gstBin "pkg-config.exe") --modversion gstreamer-1.0
