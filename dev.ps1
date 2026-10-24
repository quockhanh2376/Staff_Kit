# Staff Kit dev launcher with single-instance lifecycle & clean port hygiene

$ErrorActionPreference = "Stop"

# Ensure cargo is on PATH
if ($env:PATH -notlike "*$env:USERPROFILE\.cargo\bin*") {
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
}

function Stop-StaleStaffKitDevProcesses {
    # 1. Terminate any running staff-kit.exe processes to enforce single instance
    Get-Process -Name "staff-kit" -ErrorAction SilentlyContinue | ForEach-Object {
        try { Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue } catch {}
    }

    # 2. Terminate any process holding the Vite dev server port 5173
    $conns = Get-NetTCPConnection -LocalPort 5173 -ErrorAction SilentlyContinue
    foreach ($conn in $conns) {
        if ($conn.OwningProcess -and $conn.OwningProcess -ne $PID) {
            try {
                Stop-Process -Id $conn.OwningProcess -Force -ErrorAction SilentlyContinue
            } catch {}
        }
    }
}

# Pre-launch hygiene: ensure no orphaned instances or port collisions
Stop-StaleStaffKitDevProcesses
Start-Sleep -Milliseconds 250

try {
    npm run tauri:dev
} finally {
    # Post-run hygiene: ensure no dangling background processes on exit
    Stop-StaleStaffKitDevProcesses
}
