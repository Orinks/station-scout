; Inno Setup Script for Station Scout
; Requires Nuitka output in dist/StationScout_dir/.

#define MyAppName "Station Scout"
#ifndef MyAppVersion
  #ifexist "..\dist\version.txt"
    #define MyAppVersion ReadIni("..\dist\version.txt", "version", "value", "")
  #else
    #error Missing dist/version.txt; run installer/build_nuitka.py before compiling the installer.
  #endif
#endif
#define MyAppPublisher "Orinks"
#define MyAppURL "https://github.com/Orinks/station-scout"
#define MyAppExeName "StationScout.exe"
#define MyAppDescription "Accessible desktop internet radio explorer"

[Setup]
AppId={{4ED3D1AB-942D-4E30-9639-4BB68637A19C}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
AppComments={#MyAppDescription}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=StationScout_Setup_v{#MyAppVersion}
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/normal
SolidCompression=yes
LZMAUseSeparateProcess=yes
LZMANumBlockThreads=4
PrivilegesRequired=lowest
UsePreviousPrivileges=yes
PrivilegesRequiredOverridesAllowed=commandline
WizardStyle=modern
WizardSizePercent=100
MinVersion=10.0
UninstallDisplayName={#MyAppName}
CreateUninstallRegKey=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: checkedonce
Name: "quicklaunchicon"; Description: "{cm:CreateQuickLaunchIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\dist\StationScout_dir\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Comment: "{#MyAppDescription}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon; Comment: "{#MyAppDescription}"
Name: "{userappdata}\Microsoft\Internet Explorer\Quick Launch\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: quicklaunchicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\App Paths\{#MyAppExeName}"; ValueType: string; ValueName: ""; ValueData: "{app}\{#MyAppExeName}"; Flags: uninsdeletekey

[UninstallDelete]
Type: files; Name: "{app}\*.log"
Type: files; Name: "{app}\*.pyc"
Type: dirifempty; Name: "{app}\__pycache__"
