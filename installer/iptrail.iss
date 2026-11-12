; IPTrail - Inno Setup script.
; One file installs the app the first time and updates it later:
;   * first run: asks for the install folder (the app AND its settings, history and logs go there)
;   * later runs and in-app updates (/UPDATE): reuse the remembered folder without asking
; Build: iscc /DAppVersion=X.Y.Z /DSourceDir=path\to\target\release installer\iptrail.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceDir
  #define SourceDir "..\target\release"
#endif

#define AppName "IPTrail"
#define AppExe "IPTrail.exe"
#define SetupName "IPTrail-setup"
; Marks the install folder; the app keeps its data next to it (src/install).
#define Marker ".iptrail-install"

[Setup]
; AppId never changes: installing over an existing copy upgrades it in place, in the same folder.
AppId={{F4E7244A-5944-4D45-8F13-4E197C3168A1}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Akulav
AppPublisherURL=https://github.com/Akulav/ip-change-logger
AppSupportURL=https://github.com/Akulav/ip-change-logger/issues
AppUpdatesURL=https://github.com/Akulav/ip-change-logger/releases
AppCopyright=Made by Akulav
VersionInfoVersion={#AppVersion}
VersionInfoCompany=Akulav
VersionInfoDescription={#AppName} Setup
VersionInfoProductName={#AppName}
PrivilegesRequired=lowest
DefaultDirName={localappdata}\Programs\IPTrail
; Ask for the folder on the first install only; updates reuse the previous one.
DisableDirPage=auto
UsePreviousAppDir=yes
DirExistsWarning=no
DisableProgramGroupPage=yes
DefaultGroupName={#AppName}
OutputBaseFilename={#SetupName}
OutputDir=..\dist
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
WizardStyle=modern
CloseApplications=force
RestartApplications=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
Compression=lzma2/ultra64
SolidCompression=yes
LicenseFile=..\LICENSE

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
english.DeleteData=Also delete your settings, IP history and change log?%n%nChoose No to keep them for a future reinstall.
english.LaunchApp=Launch IPTrail
english.DataNote=IPTrail and everything it saves (settings, IP history, change log) will be stored in this folder.

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#SourceDir}\iptrail.exe"; DestDir: "{app}"; DestName: "{#AppExe}"; Flags: ignoreversion
Source: "{#SourceDir}\iptrail-cli.exe"; DestDir: "{app}"; DestName: "iptrail-cli.exe"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"; WorkingDir: "{app}"; Comment: "Watch your public IP - made by Akulav"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; WorkingDir: "{app}"; Comment: "Watch your public IP - made by Akulav"; Tasks: desktopicon

[Run]
; Interactive install: the usual "Launch" checkbox.
Filename: "{app}\{#AppExe}"; Parameters: "--installed"; WorkingDir: "{app}"; Description: "{cm:LaunchApp}"; Flags: nowait postinstall skipifsilent
; Silent in-app update (/UPDATE /FROM=<old version>): start the new version again.
Filename: "{app}\{#AppExe}"; Parameters: "--updated-from ""{param:FROM|}"""; WorkingDir: "{app}"; Flags: nowait; Check: IsUpdate

[UninstallDelete]
Type: files; Name: "{app}\{#Marker}"
Type: files; Name: "{app}\IPTrail.old.exe"
Type: files; Name: "{app}\IPTrail.new.exe"
; The downloaded update setup (in-app updates keep it in the app's own folder).
Type: filesandordirs; Name: "{app}\update"

[Code]
function IsUpdate(): Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
    if CompareText(ParamStr(I), '/UPDATE') = 0 then
      Result := True;
end;

procedure InitializeWizard();
begin
  WizardForm.SelectDirLabel.Caption := WizardForm.SelectDirLabel.Caption + #13#10 + CustomMessage('DataNote');
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    SaveStringToFile(ExpandConstant('{app}\{#Marker}'), '{#AppVersion}', False);
end;

function HasAppData(Dir: String): Boolean;
begin
  Result := FileExists(Dir + '\settings.json') or FileExists(Dir + '\history.jsonl')
    or FileExists(Dir + '\ip-changes.log') or DirExists(Dir + '\cache');
end;

{ Only what the app itself creates, never anything else the user keeps in that folder. }
procedure DeleteAppData(Dir: String);
begin
  DelTree(Dir + '\cache', True, True, True);
  DelTree(Dir + '\update', True, True, True);
  DeleteFile(Dir + '\history.jsonl');
  DeleteFile(Dir + '\ip-changes.log');
  DeleteFile(Dir + '\settings.json');
  RemoveDir(Dir);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppDir: String;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    AppDir := ExpandConstant('{app}');
    { Never delete data when silent. }
    if (not UninstallSilent) and HasAppData(AppDir) then
      if MsgBox(CustomMessage('DeleteData'), mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES then
        DeleteAppData(AppDir);
    RemoveDir(AppDir);
  end;
end;
