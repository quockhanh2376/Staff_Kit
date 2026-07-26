$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$skillsRoot = Join-Path $repo '.agents\skills'
$configPath = Join-Path $repo '.codex\config.toml'
$required = @(
    'using-superpowers','brainstorming','systematic-debugging','using-git-worktrees',
    'writing-plans','executing-plans','subagent-driven-development',
    'dispatching-parallel-agents','test-driven-development','requesting-code-review',
    'receiving-code-review','verification-before-completion','finishing-a-development-branch'
)
$approvedTools = @('headroom_stats','headroom_compress','headroom_retrieve')
$eccNames = @('staffkit-research-first','staffkit-security-review','staffkit-architecture-review','staffkit-delivery-verification')
$failures = [System.Collections.Generic.List[string]]::new()
$configHashBefore = (Get-FileHash $configPath -Algorithm SHA256).Hash

$activeNames = @()
Get-ChildItem $skillsRoot -Directory | ForEach-Object {
    $manifest = Join-Path $_.FullName 'SKILL.md'
    if (Test-Path -LiteralPath $manifest) {
        $match = Select-String -LiteralPath $manifest -Pattern '^name:\s*(.+)$' | Select-Object -First 1
        if ($match) { $activeNames += $match.Matches[0].Groups[1].Value.Trim() }
    }
}
foreach ($name in $required) {
    if (@($activeNames | Where-Object { $_ -eq $name }).Count -ne 1) {
        $failures.Add("Superpowers skill count is not one: $name")
    }
}
foreach ($name in $eccNames) {
    if (@($activeNames | Where-Object { $_ -eq $name }).Count -ne 1) {
        $failures.Add("ECC Lite skill count is not one: $name")
    }
}
if (@($activeNames | Where-Object { $_ -eq 'staffkit-focused-output' }).Count -ne 1) {
    $failures.Add('focused-output count is not one')
}
if (@($activeNames | Group-Object | Where-Object Count -gt 1).Count) {
    $failures.Add('duplicate active skill frontmatter names exist')
}

$config = Get-Content $configPath -Raw
if (($config -split '\[mcp_servers\.headroom\]').Count -ne 2) {
    $failures.Add('Headroom MCP registration count is not one')
}
foreach ($tool in $approvedTools) {
    $block = [regex]::Escape("[mcp_servers.headroom.tools.$tool]")
    if ([regex]::Matches($config, $block).Count -ne 1) {
        $failures.Add("approval block count is not one: $tool")
    }
}
if ([regex]::Matches($config, 'approval_mode\s*=\s*"approve"').Count -ne 3) {
    $failures.Add('approved Headroom tool count is not three')
}
$declaredApprovalTools = [regex]::Matches($config, '\[mcp_servers\.headroom\.tools\.([^\]]+)\]\s*\r?\napproval_mode\s*=\s*"approve"') |
    ForEach-Object { $_.Groups[1].Value }
if (@($declaredApprovalTools | Where-Object { $_ -notin $approvedTools }).Count) {
    $failures.Add('an unapproved Headroom tool has approval_mode=approve')
}
foreach ($tool in $approvedTools) {
    if ([regex]::Matches($config, '"' + [regex]::Escape($tool) + '"').Count -lt 1) {
        $failures.Add("enabled_tools is missing: $tool")
    }
}
if ($config -match 'tokensave|serena') { $failures.Add('tokensave or Serena registration exists') }

python -c "import tomllib, pathlib; tomllib.loads(pathlib.Path(r'$configPath').read_text(encoding='utf-8'))" 2>$null
if ($LASTEXITCODE -ne 0) { $failures.Add('.codex/config.toml does not parse') }

foreach ($runtimePath in @('.codex-runtime','.headroom-venv','.tokensave')) {
    if (-not (git -C $repo check-ignore "$runtimePath/" 2>$null)) { $failures.Add("$runtimePath is not ignored") }
    if (git -C $repo ls-files $runtimePath | Select-String '.') { $failures.Add("$runtimePath is tracked") }
}
if (-not (git -C $repo ls-files daily_log.md)) { $failures.Add('daily_log.md is not tracked') }
$stale = git -C $repo grep -Il -E '([Ee]:\\Staff_Kit|[Dd]:\\Staff_Kit|C:\\Users\\)' -- AGENTS.md .agent .agents .codex docs scripts 2>$null
if ($stale) { $failures.Add("machine-specific path in active tooling/docs: $($stale -join ', ')") }
$legacyConflict = rg -l -i 'mcp_sequential-thinking|mcp_context7|\*\*WAIT\*\*|\*\*MUST\*\* use `sequential-thinking`' (Join-Path $repo '.agent\workflows') 2>$null
if ($legacyConflict) { $failures.Add("legacy mandatory capability remains: $($legacyConflict -join ', ')") }
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile((Join-Path $repo 'scripts\run-codex-headroom.ps1'), [ref]$null, [ref]$errors) | Out-Null
if ($errors.Count) { $failures.Add('wrapper PowerShell syntax is invalid') }

$configHashAfter = (Get-FileHash $configPath -Algorithm SHA256).Hash
if ($configHashBefore -ne $configHashAfter) { $failures.Add('static verification changed tracked config') }

if ($failures.Count) {
    Write-Output 'STATIC NOT READY'
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Output 'STATIC READY'
exit 0
