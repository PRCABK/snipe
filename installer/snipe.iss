#ifndef MyAppVersion
#define MyAppVersion "0.1.1-alpha.4"
#endif

[Setup]
AppId={{8B490E7A-F754-4C3D-88F9-6E01594F1C02}
AppName=Snipe
AppVersion={#MyAppVersion}
AppPublisher=Snipe Contributors
AppPublisherURL=https://github.com/snipe/snipe
AppSupportURL=https://github.com/snipe/snipe
AppUpdatesURL=https://github.com/snipe/snipe
DefaultDirName={localappdata}\Programs\Snipe
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
OutputDir=..\dist
OutputBaseFilename=Snipe-Setup-{#MyAppVersion}-x64
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\snipe.exe
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "chinesesimp"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加快捷方式:"; Flags: unchecked
Name: "autostart"; Description: "开机自动启动 Snipe"; GroupDescription: "运行设置:"; Flags: unchecked

[Files]
Source: "..\target\release\snipe.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion isreadme; Tasks: ; Languages: 

[Icons]
Name: "{autoprograms}\Snipe"; Filename: "{app}\snipe.exe"
Name: "{autodesktop}\Snipe"; Filename: "{app}\snipe.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Snipe"; ValueData: """{app}\snipe.exe"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\snipe.exe"; Description: "立即运行 Snipe"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{app}\snipe.exe"; Parameters: "--shutdown-for-update"; Flags: runhidden
