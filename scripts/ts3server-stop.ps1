<#
.SYNOPSIS
    Stops local TeamSpeak 3.13.9 test server running via WSL or Docker.
.DESCRIPTION
    Terminates ts3server process in WSL (using pid_file or process signal)
    or stops and removes the Docker container ts3server-test.
#>

[CmdletBinding()]
param(
    [ValidateSet("auto", "wsl", "docker")]
    [string]$Backend = "auto",

    [string]$Distro = "Ubuntu-24.04",
    [string]$ServerDir = "D:\Dev\ts3-rust\.scratch\teamspeak3-server_linux_amd64",
    [int]$WaitSeconds = 10
)

$ErrorActionPreference = "Continue"

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

$stopped = $false

# Try stopping Docker container if requested or in auto mode
if ($Backend -eq "docker" -or ($Backend -eq "auto" -and (Test-DockerAvailable))) {
    $containerName = "ts3server-test"
    $running = & docker.exe ps -q -f "name=$containerName" 2>$null
    if ($running) {
        Write-Host "[ts3server-stop] Stopping Docker container $containerName..."
        & docker.exe stop $containerName | Out-Null
        & docker.exe rm -f $containerName | Out-Null
        Write-Host "[ts3server-stop] Docker container stopped and removed."
        $stopped = $true
    }
}

# Try stopping WSL process if requested or in auto mode
if (-not $stopped -and ($Backend -eq "wsl" -or ($Backend -eq "auto" -and (Test-WslAvailable)))) {
    $wslServerDir = Convert-ToWslPath $ServerDir
    $wslStopCmd = "cd '$wslServerDir' && if [ -f ts3server.pid ]; then kill -TERM `$(cat ts3server.pid) 2>/dev/null; rm -f ts3server.pid; fi; pkill -TERM -f ts3server 2>/dev/null"
    Write-Host "[ts3server-stop] Sending SIGTERM to ts3server in WSL..."
    & wsl.exe -d $Distro -- bash -c $wslStopCmd
    $stopped = $true
    Write-Host "[ts3server-stop] Stop signal sent to WSL ts3server process."
}

if (-not $stopped) {
    Write-Host "[ts3server-stop] Neither WSL nor Docker process was actively running or accessible."
}
