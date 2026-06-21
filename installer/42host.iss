#define MyAppName "42Host"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "Frierr"
#define MyAppExeName "42host.exe"

[Setup]
AppId={{9A092136-8622-493E-A39C-A93425827C5B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={localappdata}\Programs\42Host
DefaultGroupName=42Host
DisableProgramGroupPage=yes
OutputDir=..\installer-output
OutputBaseFilename=42HostSetup
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\bin\{#MyAppExeName}
CloseApplications=force
SetupLogging=yes

[Languages]
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Создать ярлык на рабочем столе"; GroupDescription: "Дополнительно:"; Flags: unchecked
Name: "java"; Description: "Скачать Java 21 для серверов Minecraft (рекомендуется)"; GroupDescription: "Компоненты:"; Flags: checkedonce

[Files]
Source: "..\dist\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "install-java.ps1"; DestDir: "{tmp}"; Flags: dontcopy

[Icons]
Name: "{autoprograms}\42Host"; Filename: "{app}\bin\{#MyAppExeName}"; WorkingDir: "{app}\bin"
Name: "{autodesktop}\42Host"; Filename: "{app}\bin\{#MyAppExeName}"; WorkingDir: "{app}\bin"; Tasks: desktopicon

[Run]
Filename: "{app}\bin\gio-querymodules.exe"; Parameters: """{app}\lib\gio\modules"""; Flags: runhidden waituntilterminated; Check: FileExists(ExpandConstant('{app}\bin\gio-querymodules.exe'))
Filename: "{app}\bin\gdk-pixbuf-query-loaders.exe"; Parameters: "--update-cache"; Flags: runhidden waituntilterminated; Check: FileExists(ExpandConstant('{app}\bin\gdk-pixbuf-query-loaders.exe'))
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoLogo -NoProfile -ExecutionPolicy Bypass -File ""{tmp}\install-java.ps1"" -InstallDir ""{app}"""; StatusMsg: "Скачиваем и настраиваем Java 21..."; Flags: runhidden waituntilterminated; Tasks: java
Filename: "{app}\bin\{#MyAppExeName}"; Description: "Запустить 42Host"; WorkingDir: "{app}\bin"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}\runtime"

[Code]
procedure InitializeWizard;
begin
  ExtractTemporaryFile('install-java.ps1');
end;
