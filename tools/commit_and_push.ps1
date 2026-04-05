param(
    [string]$Message = "chore: update marketing site and fly deployment flow",
    [switch]$DeployFly,
    [string]$AppName = "mermaduckle"
)

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

Push-Location $repoRoot
try {
    $status = git status --porcelain
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to read git status."
    }

    if ($status) {
        git add -A
        if ($LASTEXITCODE -ne 0) {
            throw "git add failed."
        }

        git commit -m $Message
        if ($LASTEXITCODE -ne 0) {
            throw "git commit failed."
        }
    } else {
        Write-Output "No local changes to commit."
    }

    git push
    if ($LASTEXITCODE -ne 0) {
        throw "git push failed."
    }

    if ($DeployFly.IsPresent) {
        & (Join-Path $PSScriptRoot "deploy_fly.ps1") -AppName $AppName -RemoteOnly
        if ($LASTEXITCODE -ne 0) {
            throw "Fly deploy failed."
        }
    }
} finally {
    Pop-Location
}
