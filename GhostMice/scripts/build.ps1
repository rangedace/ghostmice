[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'Rust stable (édition 2024) et un éditeur de liens Windows sont nécessaires pour compiler.'
    }
    & cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw "cargo build a échoué : $LASTEXITCODE" }
    $exe = Join-Path $root 'target\release\ghostmice.exe'
    Write-Host "Exécutable : $exe ($((Get-Item -LiteralPath $exe).Length) octets)"
    $compiler = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    $iscc = if ($compiler) { $compiler.Source } else { $null }
    if (-not $iscc) {
        $candidates = @(
            "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
            "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
            "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
        )
        $iscc = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    }
    if ($iscc) {
        & $iscc (Join-Path $root 'installer\GhostMice.iss')
        if ($LASTEXITCODE -ne 0) { throw "Inno Setup a échoué : $LASTEXITCODE" }
    } else {
        Write-Warning 'Inno Setup 6 absent : exécutable compilé, installateur non généré.'
    }
} finally { Pop-Location }
