; Inno Setup script for segotep-digital (Windows x64).
; Build: ISCC.exe /DMyAppVersion=<version> installer\segotep-digital.iss
; Version is injected by CI via /D; falls back to 0.1.0 for local builds.

#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif

#define MyAppName "Segotep Digital"
#define MyAppPublisher "Delnegend"
#define MyAppExeName "segotep-digital.exe"
#define MyAppId "{7C5F0D8E-8B9A-4E2D-9C1B-0A1B2C3D4E5F}"

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=admin
OutputDir=output
OutputBaseFilename=segotep-digital-v{#MyAppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayName={#MyAppName}
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "registerservice"; Description: "Register the Segotep Digital background service (auto-starts on boot)"; Flags: checkedonce

[Files]
Source: "..\target\x86_64-pc-windows-msvc\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"

[Run]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--install-service"; Tasks: registerservice; Flags: runhidden waituntilterminated; StatusMsg: "Registering Segotep Digital Windows service..."

[UninstallRun]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--uninstall-service"; Flags: runhidden waituntilterminated