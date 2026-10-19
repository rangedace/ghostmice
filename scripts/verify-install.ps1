[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$setup = Join-Path $root 'dist\GhostMice-Setup-0.1.0.exe'
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{2DC53565-A814-4C88-BCB0-786B1C3B34A1}_is1'
$dir = Join-Path $env:LOCALAPPDATA 'GhostMice-Installation-Test'
$exe = Join-Path $dir 'ghostmice.exe'
function Assert($condition, $message) {
    if (-not $condition) { throw $message }
    Write-Output "PASS $message"
}
function AgentProcesses {
    @(Get-Process ghostmice -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe })
}
function RunHidden($file, $arguments) {
    $process = Start-Process -FilePath $file -ArgumentList $arguments -WindowStyle Hidden -PassThru
    $process.WaitForExit()
    Assert ($process.ExitCode -eq 0) "exit=0 : $file $arguments"
}
$existing = Get-ItemProperty -LiteralPath $runKey -Name GhostMice -ErrorAction SilentlyContinue
if ($existing -or (Test-Path -LiteralPath $uninstallKey) -or (Test-Path -LiteralPath $dir) -or (Get-Process ghostmice -ErrorAction SilentlyContinue)) {
    throw 'Test refusé : installation, répertoire de test ou agent GhostMice préexistant.'
}
if (-not (Test-Path -LiteralPath $setup)) { throw 'Construire l''installateur avant ce test.' }
try {
    RunHidden $setup @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', ('/DIR="' + $dir + '"'))
    Start-Sleep -Seconds 2
    Assert (Test-Path -LiteralPath $exe) 'exécutable installé'
    $command = (Get-ItemProperty -LiteralPath $runKey -Name GhostMice).GhostMice
    Assert ($command -eq ('"' + $exe + '"')) 'commande HKCU Run exacte et entre guillemets'
    Assert ((AgentProcesses).Count -eq 1) 'une instance après installation'
    RunHidden $exe @('--test')
    Assert ((AgentProcesses).Count -eq 1) 'doublon refusé sans seconde instance'
    RunHidden $exe @('--stop')
    Start-Sleep -Milliseconds 500
    Assert ((AgentProcesses).Count -eq 0) 'arrêt coopératif sans processus restant'
    # Execute exactly the registered command, without logging off the user's session.
    Start-Process -FilePath $command.Trim('"') -WindowStyle Hidden
    Start-Sleep -Seconds 1
    Assert ((AgentProcesses).Count -eq 1) 'relance via commande Run (sans vraie réouverture de session)'
    RunHidden (Join-Path $dir 'unins000.exe') @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    Start-Sleep -Seconds 1
    Assert ((AgentProcesses).Count -eq 0) 'aucun agent de cette installation après désinstallation'
    Assert (-not (Get-ItemProperty -LiteralPath $runKey -Name GhostMice -ErrorAction SilentlyContinue)) 'entrée Run supprimée'
    Assert (-not (Test-Path -LiteralPath $uninstallKey)) 'entrée de désinstallation supprimée'
    Assert (-not (Test-Path -LiteralPath $exe)) 'exécutable supprimé'
} finally {
    if (Test-Path -LiteralPath (Join-Path $dir 'unins000.exe')) {
        RunHidden (Join-Path $dir 'unins000.exe') @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
    }
}
