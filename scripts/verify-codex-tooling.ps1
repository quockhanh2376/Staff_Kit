$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$skillsRoot = Join-Path $repo '.agents\skills'
$configPath = Join-Path $repo '.codex\config.toml'
$runtimePathHelper = Join-Path $repo 'scripts\codex-runtime-path.ps1'
$searchHelper = Join-Path $repo 'scripts\search-repository-text.ps1'
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

if (-not (Test-Path -LiteralPath $searchHelper)) {
    $failures.Add('repository text-search helper is missing')
} else {
    . $searchHelper
}

if (-not (Test-Path -LiteralPath $runtimePathHelper)) {
    $failures.Add('Codex runtime-path helper is missing')
} else {
    . $runtimePathHelper
    $sampleRoot = 'C:\Staff_Kit'
    $sameRootVariant = 'c:\staff_kit\'
    $otherRoot = 'C:\Staff_Kit\.worktrees\feature-a'
    $sampleRuntime = Get-StaffKitCodexRuntimePath -RepoRoot $sampleRoot -LocalAppData $env:LOCALAPPDATA
    $sameRuntime = Get-StaffKitCodexRuntimePath -RepoRoot $sameRootVariant -LocalAppData $env:LOCALAPPDATA
    $otherRuntime = Get-StaffKitCodexRuntimePath -RepoRoot $otherRoot -LocalAppData $env:LOCALAPPDATA
    if ($sampleRuntime -cne $sameRuntime) {
        $failures.Add('same normalized repo root does not produce a stable runtime path')
    }
    if ($sampleRuntime -ceq $otherRuntime) {
        $failures.Add('different worktree roots share a runtime path')
    }
    $expectedRuntimeBase = Join-Path $env:LOCALAPPDATA 'Staff_Kit\codex-runtime'
    if (-not $sampleRuntime.StartsWith("$expectedRuntimeBase\", [System.StringComparison]::OrdinalIgnoreCase)) {
        $failures.Add('runtime path is not under the machine-local Staff Kit runtime base')
    }
    $actualRuntime = Get-StaffKitCodexRuntimePath -RepoRoot $repo -LocalAppData $env:LOCALAPPDATA
    if ($actualRuntime.StartsWith("$repo\", [System.StringComparison]::OrdinalIgnoreCase)) {
        $failures.Add('runtime path is inside the repository')
    }
    if ((Split-Path -Leaf $actualRuntime) -notmatch '^[0-9a-f]{16}$') {
        $failures.Add('runtime path does not end in a stable short SHA-256 identifier')
    }
}

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
if (-not (Get-Command Search-RepositoryText -CommandType Function -ErrorAction SilentlyContinue)) {
    $failures.Add('repository text-search helper cannot be executed')
} else {
    $staleSearch = Search-RepositoryText -RepoRoot $repo -Pattern '([Ee]:\\Staff_Kit|[Dd]:\\Staff_Kit|C:\\Users\\)' -Paths @('AGENTS.md','.agent','.agents','.codex','docs','scripts')
    if ($staleSearch.Status -eq 'Failure') { $failures.Add("stale-path search failed: $($staleSearch.Error)") }
    elseif ($staleSearch.Status -eq 'Matches') { $failures.Add("machine-specific path in active tooling/docs: $(($staleSearch.Records.Path | Sort-Object -Unique) -join ', ')") }
    $legacySearch = Search-RepositoryText -RepoRoot $repo -Pattern 'mcp_sequential-thinking|mcp_context7|\*\*WAIT\*\*|\*\*MUST\*\* use `sequential-thinking`' -Paths @('.agent\workflows') -IgnoreCase
    if ($legacySearch.Status -eq 'Failure') { $failures.Add("legacy-workflow search failed: $($legacySearch.Error)") }
    elseif ($legacySearch.Status -eq 'Matches') { $failures.Add("legacy mandatory capability remains: $(($legacySearch.Records.Path | Sort-Object -Unique) -join ', ')") }
}
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile((Join-Path $repo 'scripts\run-codex-headroom.ps1'), [ref]$null, [ref]$errors) | Out-Null
if ($errors.Count) { $failures.Add('wrapper PowerShell syntax is invalid') }
$helperErrors = $null
[System.Management.Automation.Language.Parser]::ParseFile($runtimePathHelper, [ref]$null, [ref]$helperErrors) | Out-Null
if ($helperErrors.Count) { $failures.Add('runtime-path helper PowerShell syntax is invalid') }
$searchHelperErrors = $null
[System.Management.Automation.Language.Parser]::ParseFile($searchHelper, [ref]$null, [ref]$searchHelperErrors) | Out-Null
if ($searchHelperErrors.Count) { $failures.Add('repository text-search helper PowerShell syntax is invalid') }

$configHashAfter = (Get-FileHash $configPath -Algorithm SHA256).Hash
if ($configHashBefore -ne $configHashAfter) { $failures.Add('static verification changed tracked config') }

if ($failures.Count) {
    Write-Output 'STATIC NOT READY'
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Output 'STATIC READY'
exit 0
