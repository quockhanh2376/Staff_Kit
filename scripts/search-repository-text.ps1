function Search-RepositoryText {
    param(
        [Parameter(Mandatory = $true)]
        [string] $RepoRoot,

        [Parameter(Mandatory = $true)]
        [string] $Pattern,

        [Parameter(Mandatory = $true)]
        [string[]] $Paths,

        [switch] $Literal,
        [switch] $IgnoreCase
    )

    $backend = $null
    $rg = Get-Command rg -ErrorAction SilentlyContinue
    if ($rg) {
        $backend = 'rg'
    } else {
        $git = Get-Command git -ErrorAction SilentlyContinue
        if ($git) {
            $gitStateOutput = @(& git -C $RepoRoot rev-parse --is-inside-work-tree 2>$null)
            $gitExitCode = $LASTEXITCODE
            $gitState = $gitStateOutput | Select-Object -First 1
            if ($null -eq $gitState) { $gitState = '' }
            $gitState = $gitState.ToString().Trim()
            if ($gitExitCode -eq 0 -and $gitState -eq 'true') {
                $backend = 'git grep'
            }
        }
    }
    if (-not $backend) {
        $backend = 'PowerShell Select-String'
    }

    if (-not $script:StaffKitSearchBackendReported) {
        Write-Host "Search backend: $backend"
        $script:StaffKitSearchBackendReported = $true
    }

    $records = [System.Collections.Generic.List[object]]::new()
    $errors = [System.Collections.Generic.List[string]]::new()

    function Get-RelativeRepositoryPath {
        param([string] $Path)
        $rootFull = [System.IO.Path]::GetFullPath($RepoRoot).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
        $pathFull = [System.IO.Path]::GetFullPath($Path)
        if ($pathFull.StartsWith($rootFull, [System.StringComparison]::OrdinalIgnoreCase)) {
            return $pathFull.Substring($rootFull.Length)
        }
        return $pathFull
    }

    function Add-SearchRecord {
        param([string] $Line)
        if ($Line -match '^(.*?):(\d+):(.*)$') {
            $path = $matches[1]
            $lineNumber = [int] $matches[2]
            $text = $matches[3]
            if ([System.IO.Path]::IsPathRooted($path)) {
                $path = Get-RelativeRepositoryPath -Path $path
            }
            $records.Add([pscustomobject]@{
                Path = $path.Replace('\', '/')
                LineNumber = $lineNumber
                Line = $text
            })
        } else {
            $errors.Add("unparseable search output: $Line")
        }
    }

    if ($backend -eq 'rg') {
        $rgArgs = @('--json', '--hidden', '-I')
        if ($Literal) { $rgArgs += '-F' }
        if ($IgnoreCase) { $rgArgs += '-i' }
        $rgArgs += @('--glob', '!.git/**', '--glob', '!node_modules/**', '--glob', '!target/**',
            '--glob', '!src-tauri/target/**', '--glob', '!.headroom-venv/**',
            '--glob', '!.codex-runtime/**', '--glob', '!.tokensave/**', '--', $Pattern)
        $rgArgs += $Paths
        Push-Location -LiteralPath $RepoRoot
        try {
            $output = @(& rg @rgArgs 2>&1)
            $exitCode = $LASTEXITCODE
        } finally {
            Pop-Location
        }
        if ($exitCode -gt 1) {
            return [pscustomobject]@{ Status = 'Failure'; Backend = $backend; Records = @(); Error = ($output -join ' ') }
        }
        foreach ($line in $output) {
            try {
                $json = [string] $line | ConvertFrom-Json
                if ($json.type -eq 'match') {
                    $path = $json.data.path.text
                    $lineNumber = $json.data.line_number
                    $lineText = $json.data.lines.text.TrimEnd("`r", "`n")
                    $records.Add([pscustomobject]@{
                        Path = $path.Replace('\', '/')
                        LineNumber = $lineNumber
                        Line = $lineText
                    })
                }
            } catch {
                $errors.Add("unparseable rg JSON output: $line")
            }
        }
    } elseif ($backend -eq 'git grep') {
        $gitArgs = @('-C', $RepoRoot, 'grep', '-n', '-I')
        if ($Literal) { $gitArgs += '-F' }
        if ($IgnoreCase) { $gitArgs += '-i' }
        $gitArgs += @('-e', $Pattern, '--')
        $gitArgs += $Paths
        $output = @(& git @gitArgs 2>&1)
        $exitCode = $LASTEXITCODE
        if ($exitCode -gt 1) {
            return [pscustomobject]@{ Status = 'Failure'; Backend = $backend; Records = @(); Error = ($output -join ' ') }
        }
        foreach ($line in $output) { Add-SearchRecord -Line ([string] $line) }
    } else {
        foreach ($relativePath in $Paths) {
            $absolutePath = Join-Path $RepoRoot $relativePath
            if (-not (Test-Path -LiteralPath $absolutePath)) {
                $errors.Add("search path does not exist: $relativePath")
                continue
            }
            $files = if ((Get-Item -LiteralPath $absolutePath).PSIsContainer) {
                Get-ChildItem -LiteralPath $absolutePath -File -Recurse -Force |
                    Where-Object {
                        $_.FullName -notmatch '[\\/](\.git|node_modules|target|src-tauri[\\/]target|\.headroom-venv|\.codex-runtime|\.tokensave)([\\/]|$)'
                    }
            } else {
                @(Get-Item -LiteralPath $absolutePath)
            }
            foreach ($file in $files) {
                $bytes = Get-Content -LiteralPath $file.FullName -Encoding Byte -TotalCount 4096 -ErrorAction Stop
                if ($bytes -contains 0) { continue }
                $selectArgs = @{ LiteralPath = $file.FullName; Pattern = $Pattern; AllMatches = $true }
                if ($Literal) { $selectArgs.SimpleMatch = $true }
                if ($IgnoreCase) { $selectArgs.CaseSensitive = $false } else { $selectArgs.CaseSensitive = $true }
                $matches = Select-String @selectArgs
                foreach ($match in $matches) {
                    $records.Add([pscustomobject]@{
                        Path = (Get-RelativeRepositoryPath -Path $file.FullName).Replace('\', '/')
                        LineNumber = $match.LineNumber
                        Line = $match.Line
                    })
                }
            }
        }
    }

    if ($errors.Count -gt 0) {
        return [pscustomobject]@{ Status = 'Failure'; Backend = $backend; Records = @($records); Error = ($errors -join '; ') }
    }
    $status = if ($records.Count -gt 0) { 'Matches' } else { 'NoMatch' }
    return [pscustomobject]@{ Status = $status; Backend = $backend; Records = @($records); Error = $null }
}
