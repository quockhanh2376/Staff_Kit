$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$skillsRoot = Join-Path $repo '.agents\skills'
$required = @(
    'using-superpowers','brainstorming','systematic-debugging','using-git-worktrees',
    'writing-plans','executing-plans','subagent-driven-development',
    'dispatching-parallel-agents','test-driven-development','requesting-code-review',
    'receiving-code-review','verification-before-completion','finishing-a-development-branch'
)
$failures = [System.Collections.Generic.List[string]]::new()

foreach ($name in $required) {
    $path = Join-Path $skillsRoot "$name\SKILL.md"
    if (-not (Test-Path -LiteralPath $path)) { $failures.Add("missing Superpowers skill: $name") }
    elseif ((Select-String -LiteralPath $path -Pattern '^name:' -AllMatches).Count -ne 1) { $failures.Add("invalid frontmatter: $name") }
}
$ecc = Get-ChildItem $skillsRoot -Directory -Filter 'staffkit-*' | Where-Object { $_.Name -in @('staffkit-research-first','staffkit-security-review','staffkit-architecture-review','staffkit-delivery-verification') }
if ($ecc.Count -ne 4) { $failures.Add("ECC Lite count is $($ecc.Count), expected 4") }
if (-not (Test-Path (Join-Path $skillsRoot 'staffkit-focused-output\SKILL.md'))) { $failures.Add('missing focused-output') }
if (Test-Path (Join-Path $repo '.tokensave')) { $failures.Add('project .tokensave exists') }
if (git -C $repo ls-files .tokensave .codex-runtime | Select-String '.') { $failures.Add('runtime state is tracked') }
if (-not (git -C $repo ls-files daily_log.md)) { $failures.Add('daily_log.md is not tracked') }
$config = Get-Content (Join-Path $repo '.codex\config.toml') -Raw
if (($config -split '\[mcp_servers\.headroom\]').Count -ne 2) { $failures.Add('Headroom MCP registration count is not one') }
if ($config -match 'tokensave|serena|learn|memory|E:\\Staff_Kit') { $failures.Add('unapproved or stale config reference') }
$trackedText = git -C $repo grep -Il 'E:\\Staff_Kit' -- AGENTS.md docs/CODEX_WORKFLOW.md .codex scripts .agents .agent .gitignore 2>$null
if ($trackedText) { $failures.Add("stale path in tracked tooling: $($trackedText -join ', ')") }
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile((Join-Path $repo 'scripts\run-codex-headroom.ps1'), [ref]$null, [ref]$errors) | Out-Null
if ($errors.Count) { $failures.Add('wrapper PowerShell syntax is invalid') }

if ($failures.Count) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Output 'READY: Staff Kit Codex tooling static checks passed.'
exit 0
