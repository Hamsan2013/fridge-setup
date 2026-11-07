  ; FRIDGE Programming Language — Windows installer
; Build with Inno Setup 6:  iscc installer\fridge.iss

#define MyAppName "FRIDGE Programming Language"
#define MyAppVersion "0.1.0-alpha"
#define MyAppPublisher "KiterOS"
#define MyAppExeName "fridge.exe"

[Setup]
AppId={{4C9E5B7A-3F2D-4A1B-9C6E-FRIDGE0001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\FRIDGE
DefaultGroupName=FRIDGE
OutputDir=output
OutputBaseFilename=fridge-setup-{#MyAppVersion}
SetupIconFile=..\core\resources\fridge.ico
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=admin
ChangesEnvironment=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}
; Policy: this installer NEVER creates a desktop shortcut. Do not add one.

[Files]
Source: "..\core\target\release\fridge.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\core\resources\fridge.ico"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\examples\*"; DestDir: "{app}\examples"; Flags: recursesubdirs ignoreversion

[Registry]
; .frg file association — every FRIDGE file on this computer shows the FRIDGE icon
Root: HKCR; Subkey: ".frg"; ValueType: string; ValueName: ""; ValueData: "Fridge.Source"; Flags: uninsdeletekey
Root: HKCR; Subkey: ".frg"; ValueType: string; ValueName: "Content Type"; ValueData: "text/plain"; Flags: uninsdeletevalue
Root: HKCR; Subkey: "Fridge.Source"; ValueType: string; ValueName: ""; ValueData: "FRIDGE source file"; Flags: uninsdeletekey
Root: HKCR; Subkey: "Fridge.Source\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\fridge.ico,0"
Root: HKCR; Subkey: "Fridge.Source\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\fridge.exe"" run ""%1"""
Root: HKCR; Subkey: "Fridge.Source\shell\check\command"; ValueType: string; ValueName: ""; ValueData: """{app}\fridge.exe"" check ""%1"""
Root: HKCR; Subkey: "Fridge.Source\shell\check"; ValueType: string; ValueName: ""; ValueData: "Check with FRIDGE"

[Icons]
; Start Menu only — no desktop shortcuts by project policy
Name: "{group}\FRIDGE README"; Filename: "{app}\README.md"
Name: "{group}\FRIDGE Examples"; Filename: "{app}\examples"
Name: "{group}\Uninstall FRIDGE"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\{#MyAppExeName}"; Parameters: "version"; \
    Description: "Show FRIDGE version"; Flags: postinstall nowait skipifsilent

[Code]
const
  EnvironmentKey = 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment';

procedure EnvAddPath(Path: string);
var
  Paths: string;
begin
  if not RegQueryStringValue(HKLM, EnvironmentKey, 'Path', Paths) then
    Paths := '';
  if Pos(';' + Uppercase(Path) + ';', ';' + Uppercase(Paths) + ';') > 0 then
    exit;
  RegWriteStringValue(HKLM, EnvironmentKey, 'Path', Paths + ';' + Path + ';');
end;

procedure EnvRemovePath(Path: string);
var
  Paths: string;
  P: Integer;
begin
  if not RegQueryStringValue(HKLM, EnvironmentKey, 'Path', Paths) then
    exit;
  P := Pos(';' + Uppercase(Path) + ';', ';' + Uppercase(Paths) + ';');
  if P = 0 then
    exit;
  Delete(Paths, P, Length(Path) + 1);
  RegWriteStringValue(HKLM, EnvironmentKey, 'Path', Paths);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    EnvAddPath(ExpandConstant('{app}'));
end;

procedure CurUninstallStepChanged(CurStep: TUninstallStep);
begin
  if CurStep = usPostUninstall then
    EnvRemovePath(ExpandConstant('{app}'));
end;
