# NCR AI Desk - unified launcher (replaces scripts/)
param(
    [ValidateSet("all", "docs", "api", "web", "stop", "qwen")]
    [string]$Action = "all"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = $PSScriptRoot
$QwenPort = 8092
$ApiPort = 8090
$WebPort = 8080

function Import-DeskEnv {
    $envFile = Join-Path $ProjectRoot ".env"
    if (-not (Test-Path $envFile)) { return }
    Get-Content $envFile | ForEach-Object {
        $line = $_.Trim()
        if ($line -eq "" -or $line.StartsWith("#")) { return }
        $idx = $line.IndexOf("=")
        if ($idx -lt 1) { return }
        $name = $line.Substring(0, $idx).Trim()
        $value = $line.Substring($idx + 1).Trim()
        if ($name) { Set-Item -Path "Env:$name" -Value $value }
    }
}

function Get-PortPids {
    param([int]$Port)
    $pids = @()
    $lines = netstat -ano -p tcp | Select-String ":$Port\s+.*LISTENING\s+(\d+)$"
    foreach ($line in $lines) {
        $match = [regex]::Match($line.Line, "LISTENING\s+(\d+)$")
        if ($match.Success) { $pids += [int]$match.Groups[1].Value }
    }
    return $pids | Select-Object -Unique
}

function Stop-DeskPorts {
    foreach ($port in $QwenPort, $ApiPort, $WebPort) {
        foreach ($procId in (Get-PortPids -Port $port)) {
            if ($procId -gt 0) {
                Write-Host "Stopping port $port (PID $procId)"
                Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
            }
        }
    }
}

function Ensure-Jdk {
    $toolsDir = Join-Path $ProjectRoot ".tools"
    $jdkRoot = Join-Path $toolsDir "jdk17"
    $java = Get-ChildItem -Path $jdkRoot -Filter "java.exe" -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -like "*\bin\java.exe" } | Select-Object -First 1
    if ($java) {
        return (Split-Path -Parent (Split-Path -Parent $java.FullName))
    }
    if ((Get-Command java -ErrorAction SilentlyContinue) -and (Get-Command javac -ErrorAction SilentlyContinue)) {
        return ""
    }
    New-Item -ItemType Directory -Force -Path $toolsDir | Out-Null
    $zipPath = Join-Path $toolsDir "jdk17.zip"
    if (-not (Test-Path $zipPath)) {
        Write-Host "Downloading portable Java 17..."
        curl.exe -L "https://api.adoptium.net/v3/binary/latest/17/ga/windows/x64/jdk/hotspot/normal/eclipse?project=jdk" -o $zipPath
    }
    Write-Host "Extracting Java 17..."
    New-Item -ItemType Directory -Force -Path $jdkRoot | Out-Null
    Expand-Archive -Force $zipPath $jdkRoot
    $java = Get-ChildItem -Path $jdkRoot -Filter "java.exe" -Recurse |
        Where-Object { $_.FullName -like "*\bin\java.exe" } | Select-Object -First 1
    if (-not $java) { throw "java.exe not found after JDK extract." }
    return (Split-Path -Parent (Split-Path -Parent $java.FullName))
}

function Ensure-Maven {
    if (Get-Command mvn -ErrorAction SilentlyContinue) { return "" }
    $toolsDir = Join-Path $ProjectRoot ".tools"
    $mavenRoot = Join-Path $toolsDir "maven"
    $maven = Get-ChildItem -Path $mavenRoot -Filter "mvn.cmd" -Recurse -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($maven) {
        return (Split-Path -Parent (Split-Path -Parent $maven.FullName))
    }
    New-Item -ItemType Directory -Force -Path $toolsDir | Out-Null
    $zipPath = Join-Path $toolsDir "apache-maven-3.9.9-bin.zip"
    if (-not (Test-Path $zipPath)) {
        Write-Host "Downloading portable Maven..."
        curl.exe -L "https://archive.apache.org/dist/maven/maven-3/3.9.9/binaries/apache-maven-3.9.9-bin.zip" -o $zipPath
    }
    Write-Host "Extracting Maven..."
    New-Item -ItemType Directory -Force -Path $mavenRoot | Out-Null
    Expand-Archive -Force $zipPath $mavenRoot
    $maven = Get-ChildItem -Path $mavenRoot -Filter "mvn.cmd" -Recurse | Select-Object -First 1
    if (-not $maven) { throw "mvn.cmd not found after Maven extract." }
    return (Split-Path -Parent (Split-Path -Parent $maven.FullName))
}

function Ensure-Rust {
    if (Get-Command cargo -ErrorAction SilentlyContinue) { return "" }
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path (Join-Path $cargoBin "cargo.exe")) { return $cargoBin }
    Write-Host "Installing Rust (rustup)..."
    $installer = Join-Path $env:TEMP "rustup-init.exe"
    curl.exe -L "https://win.rustup.rs/x86_64" -o $installer
    & $installer -y --default-toolchain stable
    if (-not (Test-Path (Join-Path $cargoBin "cargo.exe"))) {
        throw "Rust install failed: cargo.exe not found."
    }
    return $cargoBin
}

function Start-DeskDocs {
    Import-DeskEnv
    $venv = Join-Path $ProjectRoot ".tools\qwen-venv"
    $pip = Join-Path $venv "Scripts\pip.exe"
    $requirements = Join-Path $ProjectRoot "qwen-service\requirements.txt"
    if (-not (Test-Path (Join-Path $venv "Scripts\python.exe"))) {
        Write-Host "Creating Python venv..."
        python -m venv $venv
    }
    Write-Host "Installing document service dependencies..."
    & $pip install -r $requirements
    if ($LASTEXITCODE -ne 0) { throw "pip install failed." }
    $env:DOCUMENT_BIND_HOST = "127.0.0.1"
    $env:DOCUMENT_BIND_PORT = "$QwenPort"
    Write-Host "Document service: http://127.0.0.1:$QwenPort/"
    & (Join-Path $venv "Scripts\python.exe") (Join-Path $ProjectRoot "qwen-service\server.py")
}

function Start-DeskApi {
    Import-DeskEnv
    $cargoBin = Ensure-Rust
    if ($cargoBin) {
        if ($cargoBin -is [System.Array]) { $cargoBin = $cargoBin[-1] }
        $env:Path = "$cargoBin;$env:Path"
    }
    $cargoPath = $null
    if ($cargoBin) {
        $candidate = Join-Path $cargoBin "cargo.exe"
        if (Test-Path $candidate) { $cargoPath = $candidate }
    }
    if (-not $cargoPath) {
        $cargo = Get-Command cargo -ErrorAction SilentlyContinue
        if ($cargo) { $cargoPath = $cargo.Source }
    }
    if (-not $cargoPath) { throw "cargo not found." }
    $deskDir = Join-Path $ProjectRoot "ai-service"
    $env:AI_DESK_BIND = "127.0.0.1:$ApiPort"
    Write-Host "Rust API: http://127.0.0.1:$ApiPort/"
    & $cargoPath run --manifest-path (Join-Path $deskDir "Cargo.toml")
}

function Start-DeskWeb {
    Import-DeskEnv
    New-Item -ItemType Directory -Force -Path (Join-Path $ProjectRoot ".data") | Out-Null
    $jdkHome = Ensure-Jdk
    $mavenHome = Ensure-Maven
    if ($jdkHome) {
        $env:JAVA_HOME = $jdkHome
        $env:Path = (Join-Path $jdkHome "bin") + ";" + $env:Path
    }
    $mvn = if ($mavenHome) { Join-Path $mavenHome "bin\mvn.cmd" } else { "mvn" }
    $pom = Join-Path $ProjectRoot "pom.xml"
    $jar = Join-Path $ProjectRoot "target\company-ai-desk-frontend-0.1.0.jar"
    Write-Host "Building Spring UI (mvn package)..."
    & $mvn -f $pom -q -DskipTests package
    if ($LASTEXITCODE -ne 0) { throw "Maven build failed for the web UI." }
    if (-not (Test-Path $jar)) { throw "JAR not found after build: $jar" }
    $java = if ($jdkHome) { Join-Path $jdkHome "bin\java.exe" } else { "java" }
    Write-Host "Web UI: http://127.0.0.1:$WebPort/"
    Set-Location $ProjectRoot
    & $java -jar $jar
}

function Start-DeskDocsWindow {
    Start-Process powershell -ArgumentList @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", (Join-Path $ProjectRoot "desk.ps1"),
        "-Action", "docs"
    )
}

function Start-DeskApiWindow {
    Start-Process powershell -ArgumentList @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", (Join-Path $ProjectRoot "desk.ps1"),
        "-Action", "api"
    )
}

function Start-DeskWebWindow {
    Start-Process powershell -ArgumentList @(
        "-NoProfile", "-ExecutionPolicy", "Bypass",
        "-File", (Join-Path $ProjectRoot "desk.ps1"),
        "-Action", "web"
    )
}

Import-DeskEnv

switch ($Action) {
    "stop" {
        Stop-DeskPorts
        Write-Host "Ports $QwenPort, $ApiPort, $WebPort should be free."
    }
    "docs" {
        Start-DeskDocs
    }
    "qwen" {
        Write-Host "Qwen was removed. Use: desk.ps1 -Action docs"
        Start-DeskDocs
    }
    "api" {
        Start-DeskApi
    }
    "web" {
        Start-DeskWeb
    }
    "all" {
        Write-Host "Starting NCR AI Desk..."
        Stop-DeskPorts
        Start-Sleep -Seconds 1
        if (-not $env:PERPLEXITY_API_KEY) {
            Write-Host "WARNING: PERPLEXITY_API_KEY not set - add it to .env for live chat (weather, news)."
            Write-Host "  Get a key: https://www.perplexity.ai/settings/api"
        }
        Start-DeskDocsWindow
        Start-DeskApiWindow
        Start-Sleep -Seconds 2
        New-Item -ItemType Directory -Force -Path (Join-Path $ProjectRoot ".data") | Out-Null
        Start-DeskWebWindow
        Start-Sleep -Seconds 8
        Write-Host "Tip: wait until the Web UI window shows 'Started AiDeskFrontendApplication' before opening the browser."
        Write-Host ""
        Write-Host "Open http://127.0.0.1:${WebPort}/"
        Write-Host "  API:  http://127.0.0.1:${ApiPort}/health"
        Write-Host "  Docs: http://127.0.0.1:${QwenPort}/health"
    }
}
