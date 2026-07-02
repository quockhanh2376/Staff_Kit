param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $CodexArgs
)

$ErrorActionPreference = 'Stop'

$env:TERM = 'xterm-256color'

$runtimeCodexHome = Join-Path (Get-Location) '.codex-runtime'
New-Item -ItemType Directory -Force -Path $runtimeCodexHome | Out-Null
Copy-Item .\.codex\config.toml (Join-Path $runtimeCodexHome 'config.toml') -Force

$userCodexAuth = Join-Path $env:USERPROFILE '.codex\auth.json'
if (Test-Path $userCodexAuth) {
    Copy-Item $userCodexAuth (Join-Path $runtimeCodexHome 'auth.json') -Force
}

$env:CODEX_HOME = (Resolve-Path $runtimeCodexHome).Path

$codexExe = (Get-Command codex -ErrorAction SilentlyContinue).Source
if (-not $codexExe) {
    $candidate = Get-ChildItem "$env:USERPROFILE\.vscode\extensions\openai.chatgpt-*\bin\windows-x86_64\codex.exe" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if ($candidate) {
        $codexExe = $candidate.FullName
    }
}

if (-not $codexExe) {
    throw "Could not find codex.exe. Install OpenAI Codex CLI or open Codex once in VS Code so the extension is available."
}

$codexDir = Split-Path -Parent $codexExe
$headroomScripts = (Resolve-Path .\.headroom-venv\Scripts).Path
$env:PATH = $codexDir + ';' + $headroomScripts + ';' + $env:PATH

& .\.headroom-venv\Scripts\headroom.exe wrap codex @CodexArgs
