param(
    [int]$Port = 3001,
    [int]$WaitSeconds = 60
)

Write-Output "Stopping any running mermaduckle-server processes..."
try {
    Get-Process mermaduckle-server -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
} catch {
    # fallback to taskkill if needed
    Try { taskkill /F /IM mermaduckle-server.exe /T } Catch {}
}

Write-Output "Starting server (this will stream output to this console)..."
# Start server in a new PowerShell window so this script can continue
Start-Process -NoNewWindow -FilePath "powershell" -ArgumentList "-NoProfile -Command cd \"$(Resolve-Path ..)\"; cargo run -p mermaduckle-server" -WindowStyle Normal

Write-Output "Waiting for server health on http://127.0.0.1:$Port/api/health (timeout ${WaitSeconds}s)..."
$ok = $false
for ($i=0; $i -lt $WaitSeconds; $i++) {
    try {
        $res = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/api/health" -TimeoutSec 2 -ErrorAction Stop
        if ($res.status -eq 'ok') { $ok = $true; break }
    } catch { }
    Start-Sleep -Seconds 1
}

if (-not $ok) {
    Write-Error "Server did not become healthy in time. Check server logs and retry."
    exit 2
}

Write-Output "Server healthy. Creating a one-time dev API key via tools/create_api_key.py..."
try {
    python tools/create_api_key.py
} catch {
    Write-Error "Failed to run tools/create_api_key.py. Ensure Python is installed and the server is running."
    exit 3
}

Write-Output "If a raw API key was printed above, copy it and paste into the browser console using the console snippet or run the one-liner below replacing <KEY>:\n"
Write-Output "(function(k){ localStorage.setItem('apiKey', k); window.apiKey = k; location.reload(); })('<KEY>');"

Write-Output "Committing and pushing repository changes..."
try {
    git add -A
    git commit -m "chore: automated dev run helper; README and dev port updates"
    git push
} catch {
    Write-Error "Git commit/push failed. Run tools\commit_and_push.ps1 manually if needed."
}

Write-Output "Done."
