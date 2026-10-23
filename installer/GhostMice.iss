#define AppVersion "0.1.1"

[Setup]
AppId={{2DC53565-A814-4C88-BCB0-786B1C3B34A1}
AppName=GhostMice
AppVersion={#AppVersion}
DefaultDirName={localappdata}\Programs\GhostMice
PrivilegesRequired=lowest
MinVersion=10.0
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
DisableProgramGroupPage=yes
DisableReadyPage=no
OutputDir=..\dist
OutputBaseFilename=GhostMice-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
UninstallDisplayName=GhostMice
CloseApplications=yes
CloseApplicationsFilter=ghostmice.exe
RestartApplications=no
LicenseFile=..\LICENSE
InfoBeforeFile=install-info.txt
SetupLogging=yes

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "..\target\release\ghostmice.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "GhostMice"; ValueData: """{app}\ghostmice.exe"""; Flags: uninsdeletevalue

[Run]
Filename: "{app}\ghostmice.exe"; Description: "Démarrage automatique en arrière-plan"; Flags: nowait runhidden

[Code]
function StopAgent(): Boolean;
var Code: Integer;
begin
  Result := True;
  if FileExists(ExpandConstant('{app}\ghostmice.exe')) then
    Result := Exec(ExpandConstant('{app}\ghostmice.exe'), '--stop', '', SW_HIDE, ewWaitUntilTerminated, Code) and (Code = 0);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  if not StopAgent() then
    Result := 'Impossible d''arrêter GhostMice. Fermez le processus dans cette session puis réessayez.';
end;

function InitializeUninstall(): Boolean;
begin
  Result := StopAgent();
  if not Result then
    MsgBox('Impossible d''arrêter GhostMice. Désinstallation annulée pour ne pas laisser un agent actif.', mbError, MB_OK);
end;
