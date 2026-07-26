function Get-StaffKitCodexRuntimePath {
    param(
        [Parameter(Mandatory = $true)]
        [string] $RepoRoot,

        [Parameter(Mandatory = $true)]
        [string] $LocalAppData
    )

    $absoluteRoot = [System.IO.Path]::GetFullPath($RepoRoot)
    $pathRoot = [System.IO.Path]::GetPathRoot($absoluteRoot)
    if (-not $absoluteRoot.Equals($pathRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        $absoluteRoot = $absoluteRoot.TrimEnd(
            [System.IO.Path]::DirectorySeparatorChar,
            [System.IO.Path]::AltDirectorySeparatorChar
        )
    }
    $normalizedRoot = $absoluteRoot.ToLowerInvariant()

    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        $digest = $sha256.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($normalizedRoot))
    } finally {
        $sha256.Dispose()
    }
    $repoRootHash = ([System.BitConverter]::ToString($digest) -replace '-', '').ToLowerInvariant().Substring(0, 16)

    $runtimeBase = Join-Path $LocalAppData 'Staff_Kit\codex-runtime'
    return Join-Path $runtimeBase $repoRootHash
}
