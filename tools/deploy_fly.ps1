param(
    [string]$AppName = "mermaduckle",
    [string]$ConfigPath = "fly.toml",
    [switch]$BuildOnly,
    [switch]$RemoteOnly,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ExtraArgs
)

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$flyctl = Get-Command flyctl -ErrorAction SilentlyContinue

if (-not $flyctl) {
    Write-Error "flyctl is not installed or not on PATH. Install flyctl before deploying."
    exit 1
}

$args = @("deploy", "-a", $AppName, "--config", $ConfigPath)

if ($BuildOnly.IsPresent) {
    $args += @("--build-only", "--push")
}

if ($RemoteOnly.IsPresent) {
    $args += "--remote-only"
}

if ($ExtraArgs) {
    $args += $ExtraArgs
}

Write-Output "Running: flyctl $($args -join ' ')"

Push-Location $repoRoot
try {
    & $flyctl.Source @args
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
}
