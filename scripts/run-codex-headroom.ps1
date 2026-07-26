param(
    [switch] $NoHeadroom,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $CodexArgs
)

$ErrorActionPreference = 'Stop'

# Resolve the repository from this script so the launcher is portable across
# drives and machines.
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$runtimeRoot = Join-Path $env:LOCALAPPDATA 'Staff_Kit\codex-runtime'
$runtimeConfig = Join-Path $runtimeRoot 'config.toml'
$templateConfig = Join-Path $repoRoot '.codex\config.toml'
$headroomExe = Join-Path $repoRoot '.headroom-venv\Scripts\headroom.exe'

New-Item -ItemType Directory -Force -Path $runtimeRoot | Out-Null
Copy-Item -LiteralPath $templateConfig -Destination $runtimeConfig -Force

$codexExe = (Get-Command codex -ErrorAction SilentlyContinue).Source
if (-not $codexExe) {
    $candidate = Get-ChildItem "$env:USERPROFILE\.vscode\extensions\openai.chatgpt-*\bin\windows-x86_64\codex.exe" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($candidate) { $codexExe = $candidate.FullName }
}
if (-not $codexExe) { throw "Could not find codex.exe." }

$userAuth = Join-Path $env:USERPROFILE '.codex\auth.json'
if (Test-Path -LiteralPath $userAuth) {
    Copy-Item -LiteralPath $userAuth -Destination (Join-Path $runtimeRoot 'auth.json') -Force
}

$codexDir = Split-Path -Parent $codexExe
$env:CODEX_HOME = $runtimeRoot
$env:PATH = "$codexDir;$(Split-Path -Parent $headroomExe);$env:PATH"

if ($NoHeadroom) {
    Write-Warning "Headroom compression is unavailable by request; launching Codex directly with the isolated Staff Kit runtime."
    & $codexExe @CodexArgs
    exit $LASTEXITCODE
}

if (-not (Test-Path -LiteralPath $headroomExe)) {
    Write-Warning "Headroom is unavailable; launching Codex directly with the isolated project runtime."
    & $codexExe @CodexArgs
    exit $LASTEXITCODE
}

$headroomVersion = (& $headroomExe --version 2>$null | Select-Object -First 1)
if ($headroomVersion -notmatch 'headroom, version') {
    Write-Warning "Headroom executable failed version validation; launching Codex directly."
    & $codexExe @CodexArgs
    exit $LASTEXITCODE
}

# Keep Headroom limited to proxy, retrieval and statistics. Explicitly disable
# its optional context tools, code graph, and Serena. Memory and learning are
# opt-in flags and are therefore never passed.
& $headroomExe wrap codex --no-context-tool --no-tokensave --no-serena -- @CodexArgs
if ($LASTEXITCODE -ne 0) {
    Write-Warning "Headroom launch failed; retrying Codex directly with the isolated runtime."
    & $codexExe @CodexArgs
    exit $LASTEXITCODE
}
exit 0
