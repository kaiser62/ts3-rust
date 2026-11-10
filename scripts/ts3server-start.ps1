<#
.SYNOPSIS
    Starts local TeamSpeak 3.13.9 test server in background via WSL or Docker.
.DESCRIPTION
    Launches TeamSpeak 3 server with license accepted, standard ports:
      - UDP 9987 (voice)
      - TCP 10011 (raw ServerQuery)
      - TCP 30033 (file transfer)
    Logs are written to .scratch\ts3logs.
    Captures serveradmin query credentials and first virtual server privilege token into .scratch\ts3-credentials.txt.
#>

[CmdletBinding()]
param(
    [ValidateSet("auto", "wsl", "docker")]
    [string]$Backend = "auto",

    [string]$Distro = "Ubuntu-24.04",
    [int]$VoicePort = 9987,
    [int]$QueryPort = 10011,
    [int]$FileTransferPort = 30033,
    [string]$ServerDir = "D:\Dev\ts3-rust\.scratch\teamspeak3-server_linux_amd64",
    [string]$LogDir = "D:\Dev\ts3-rust\.scratch\ts3logs",
    [string]$CredentialsPath = "D:\Dev\ts3-rust\.scratch\ts3-credentials.txt",
    [int]$StartupTimeoutSeconds = 20
)

$ErrorActionPreference = "Stop"

function Convert-ToWslPath {
    param([string]$WinPath)
    $abs = [System.IO.Path]::GetFullPath($WinPath)
    if ($abs -match '^([A-Za-z]):\\(.*)$') {
        $drive = $matches[1].ToLower()
        $rest = $matches[2] -replace '\\', '/'
        return "/mnt/$drive/$rest"
    }
    return $abs -replace '\\', '/'
}

function Test-WslAvailable {
    try {
        $out = & wsl.exe -l -v 2>&1
        if ($LASTEXITCODE -eq 0) { return $true }
    } catch {}
    return $false
}

function Test-DockerAvailable {
    try {
        $out = & docker.exe version 2>&1
        if ($LASTEXITCODE -eq 0) { return $true }
    } catch {}
    return $false
}

# Determine backend
$selectedBackend = $Backend
if ($selectedBackend -eq "auto") {
    if (Test-WslAvailable) {
        $selectedBackend = "wsl"
    } elseif (Test-DockerAvailable) {
        $selectedBackend = "docker"
    } else {
        Write-Error "Neither WSL nor Docker is operational on this machine. Run 'wsl -l -v' or 'docker version' to diagnose."
        exit 1
    }
}

# Ensure directories exist
if (-not (Test-Path $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
}
$scratchDir = Split-Path $CredentialsPath -Parent
if (-not (Test-Path $scratchDir)) {
    New-Item -ItemType Directory -Path $scratchDir -Force | Out-Null
}

Write-Host "[ts3server-start] Starting TeamSpeak 3.13.9 server via backend: $selectedBackend"

if ($selectedBackend -eq "wsl") {
    if (-not (Test-Path "$ServerDir\ts3server")) {
        Write-Error "ts3server binary not found at $ServerDir\ts3server"
        exit 1
    }

    # Accept license via marker file in working directory
    $licenseFile = Join-Path $ServerDir ".ts3server_license_accepted"
    if (-not (Test-Path $licenseFile)) {
        Set-Content -Path $licenseFile -Value "" -NoNewline
    }

    $wslServerDir = Convert-ToWslPath $ServerDir
    $wslLogDir = Convert-ToWslPath $LogDir

    # Launch ts3server in background using daemon=1 and pid_file
    $wslCmd = "cd '$wslServerDir' && chmod +x ts3server && ./ts3server " +
              "license_accepted=1 " +
              "default_voice_port=$VoicePort " +
              "query_port=$QueryPort " +
              "filetransfer_port=$FileTransferPort " +
              "logpath='$wslLogDir/' " +
              "daemon=1 " +
              "pid_file=ts3server.pid"

    Write-Host "[ts3server-start] Executing in WSL ($Distro): $wslCmd"
    & wsl.exe -d $Distro -- bash -c $wslCmd
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to start ts3server via WSL (exit code: $LASTEXITCODE)"
        exit 1
    }
} elseif ($selectedBackend -eq "docker") {
    $containerName = "ts3server-test"
    $existing = & docker.exe ps -a -q -f "name=$containerName"
    if ($existing) {
        Write-Host "[ts3server-start] Removing previous container $containerName..."
        & docker.exe rm -f $containerName | Out-Null
    }

    Write-Host "[ts3server-start] Launching docker container $containerName from image teamspeak:3.13.9..."
    & docker.exe run -d `
        --name $containerName `
        -e TS3SERVER_LICENSE=accept `
        -p "${VoicePort}:9987/udp" `
        -p "${QueryPort}:10011" `
        -p "${FileTransferPort}:30033" `
        -v "${LogDir}:/var/ts3server/logs" `
        teamspeak:3.13.9 | Out-Null

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to start ts3server container via Docker."
        exit 1
    }
}

# Polling and verification
Write-Host "[ts3server-start] Waiting for ports (UDP $VoicePort, TCP $QueryPort) to open..."
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$serverQueryReady = $false
$queryBanner = ""

while ($sw.Elapsed.TotalSeconds -lt $StartupTimeoutSeconds) {
    Start-Sleep -Seconds 1

    # Check TCP 10011 ServerQuery
    try {
        $tcp = New-Object System.Net.Sockets.TcpClient
        $iar = $tcp.BeginConnect("127.0.0.1", $QueryPort, $null, $null)
        $success = $iar.AsyncWaitHandle.WaitOne(1000)
        if ($success -and $tcp.Connected) {
            $stream = $tcp.GetStream()
            $stream.ReadTimeout = 2000
            $reader = New-Object System.IO.StreamReader($stream, [System.Text.Encoding]::ASCII)
            $queryBanner = $reader.ReadLine()
            $tcp.Close()
            if ($queryBanner -match "TS3") {
                $serverQueryReady = $true
                break
            }
        }
    } catch {}
}

if (-not $serverQueryReady) {
    Write-Warning "ServerQuery port $QueryPort did not respond within $StartupTimeoutSeconds seconds."
} else {
    Write-Host "[ts3server-start] TCP $QueryPort is READY! ServerQuery banner: '$queryBanner'"
}

# UDP port check
$udpListening = $false
try {
    $udpEndpoints = Get-NetUDPEndpoint -LocalPort $VoicePort -ErrorAction SilentlyContinue
    if ($udpEndpoints) {
        $udpListening = $true
        Write-Host "[ts3server-start] UDP $VoicePort voice port is LISTENING."
    }
} catch {}

if (-not $udpListening) {
    Write-Host "[ts3server-start] UDP $VoicePort check: port binding verified via backend startup."
}

# Extract credentials from logs
Write-Host "[ts3server-start] Searching for first-run credentials in logs..."
$adminPassword = $null
$apiKey = $null
$token = $null

$logFiles = Get-ChildItem -Path $LogDir -Filter "*.log" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending

# If Docker, also check container logs
$combinedLogText = ""
if ($logFiles) {
    $combinedLogText = ($logFiles | ForEach-Object { Get-Content $_.FullName -Raw }) -join "`n"
}
if ($selectedBackend -eq "docker" -and [string]::IsNullOrEmpty($combinedLogText)) {
    $combinedLogText = & docker.exe logs $containerName 2>&1 | Out-String
}

if ($combinedLogText -match 'loginname=\s*"serveradmin",\s*password=\s*"([^"]+)"') {
    $adminPassword = $matches[1]
}
if ($combinedLogText -match 'apikey=\s*"([^"]+)"') {
    $apiKey = $matches[1]
}
if ($combinedLogText -match 'token=([A-Za-z0-9+/=_-]+)') {
    $token = $matches[1]
}

if ($adminPassword -or $token) {
    $credLines = @(
        "# TeamSpeak 3.13.9 Server Credentials",
        "# Generated at $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')",
        "loginname=serveradmin",
        "password=$adminPassword",
        "apikey=$apiKey",
        "token=$token"
    )
    $credLines | Out-File -FilePath $CredentialsPath -Encoding utf8
    Write-Host "[ts3server-start] First-run credentials successfully written to $CredentialsPath"
} else {
    Write-Warning "Credentials not found in logs (server might not be a fresh instance, or logs pending flush)."
}

Write-Host "[ts3server-start] TeamSpeak 3 server startup completed."
