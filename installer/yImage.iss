; Inno Setup script for yImage
; Build with: iscc installer\yImage.iss
;
; Registers file associations for common image formats and makes yImage appear
; in the Windows 10/11 "Default apps" UI via RegisteredApplications.

#define AppName "yImage"
#define AppVersion "0.1.0"
#define AppPublisher "Youngmin Kim"
#define AppURL "https://ko-fi.com/youngminkim"
#define AppExeName "yimage.exe"
#define ProgId "yImage.Image.1"
#define CtxKey "yImage.ContextMenu"

[Setup]
AppId={{B1B9F85D-9C58-4E9F-9B9A-9A5C6F7E1E21}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
OutputBaseFilename=yImage-Setup-{#AppVersion}
Compression=lzma2/ultra
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64
ArchitecturesAllowed=x64
MinVersion=10.0.17763
PrivilegesRequired=admin
UninstallDisplayIcon={app}\{#AppExeName}
SetupIconFile=..\assets\icons\yimage.ico

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "korean";  MessagesFile: "compiler:Languages\Korean.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "fileassoc";   Description: "Associate common image formats with yImage"; GroupDescription: "File associations:"
Name: "ctxmenu";     Description: "Add yImage to the image right-click menu"; GroupDescription: "File associations:"

[Files]
Source: "..\target\release\yimage.exe"; DestDir: "{app}"; Flags: ignoreversion
; CJK fonts are embedded into the exe via include_bytes! — no external copy needed.
Source: "..\assets\icons\*";            DestDir: "{app}\assets\icons";  Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist
Source: "..\assets\models\*";           DestDir: "{app}\assets\models"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist
Source: "..\README.md";                 DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.ko.md";              DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.ja.md";              DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE";                   DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Registry]
; --- ProgID ----------------------------------------------------------------
Root: HKLM; Subkey: "Software\Classes\{#ProgId}"; ValueType: string; ValueName: ""; ValueData: "yImage"; Flags: uninsdeletekey; Tasks: fileassoc
Root: HKLM; Subkey: "Software\Classes\{#ProgId}\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExeName}"",0"; Tasks: fileassoc
Root: HKLM; Subkey: "Software\Classes\{#ProgId}\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExeName}"" ""%1"""; Tasks: fileassoc

; --- Per-extension OpenWithProgids so "Open With" shows yImage --------------
Root: HKLM; Subkey: "Software\Classes\.png\OpenWithProgids";  ValueType: string; ValueName: "{#ProgId}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKLM; Subkey: "Software\Classes\.jpg\OpenWithProgids";  ValueType: string; ValueName: "{#ProgId}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKLM; Subkey: "Software\Classes\.jpeg\OpenWithProgids"; ValueType: string; ValueName: "{#ProgId}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKLM; Subkey: "Software\Classes\.webp\OpenWithProgids"; ValueType: string; ValueName: "{#ProgId}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKLM; Subkey: "Software\Classes\.bmp\OpenWithProgids";  ValueType: string; ValueName: "{#ProgId}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKLM; Subkey: "Software\Classes\.gif\OpenWithProgids";  ValueType: string; ValueName: "{#ProgId}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKLM; Subkey: "Software\Classes\.tif\OpenWithProgids";  ValueType: string; ValueName: "{#ProgId}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKLM; Subkey: "Software\Classes\.tiff\OpenWithProgids"; ValueType: string; ValueName: "{#ProgId}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: fileassoc
Root: HKLM; Subkey: "Software\Classes\.avif\OpenWithProgids"; ValueType: string; ValueName: "{#ProgId}"; ValueData: ""; Flags: uninsdeletevalue; Tasks: fileassoc

; --- RegisteredApplications so Windows "Default apps" shows yImage ----------
Root: HKLM; Subkey: "Software\{#AppName}\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "{#AppName}"; Flags: uninsdeletekey; Tasks: fileassoc
Root: HKLM; Subkey: "Software\{#AppName}\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "Fast image viewer and editor"; Tasks: fileassoc
Root: HKLM; Subkey: "Software\{#AppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".png";  ValueData: "{#ProgId}"; Tasks: fileassoc
Root: HKLM; Subkey: "Software\{#AppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".jpg";  ValueData: "{#ProgId}"; Tasks: fileassoc
Root: HKLM; Subkey: "Software\{#AppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".jpeg"; ValueData: "{#ProgId}"; Tasks: fileassoc
Root: HKLM; Subkey: "Software\{#AppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".webp"; ValueData: "{#ProgId}"; Tasks: fileassoc
Root: HKLM; Subkey: "Software\{#AppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".bmp";  ValueData: "{#ProgId}"; Tasks: fileassoc
Root: HKLM; Subkey: "Software\{#AppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".gif";  ValueData: "{#ProgId}"; Tasks: fileassoc
Root: HKLM; Subkey: "Software\{#AppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".tif";  ValueData: "{#ProgId}"; Tasks: fileassoc
Root: HKLM; Subkey: "Software\{#AppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".tiff"; ValueData: "{#ProgId}"; Tasks: fileassoc
Root: HKLM; Subkey: "Software\{#AppName}\Capabilities\FileAssociations"; ValueType: string; ValueName: ".avif"; ValueData: "{#ProgId}"; Tasks: fileassoc
Root: HKLM; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "{#AppName}"; ValueData: "Software\{#AppName}\Capabilities"; Flags: uninsdeletevalue; Tasks: fileassoc

; --- Explorer right-click cascading submenu ("yImage" → actions) ------------
; Shared verb store under HKLM\Software\Classes\yImage.ContextMenu\shell\*.
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\01_open"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Open with yImage"; Flags: uninsdeletekey; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\01_open"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\01_open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExeName}"" ""%1"""; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\02_optimize"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Optimize with yImage"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\02_optimize"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\02_optimize\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExeName}"" --optimize ""%1"""; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\03_resize"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Resize with yImage"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\03_resize"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\03_resize\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExeName}"" --resize ""%1"""; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\04_convert"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Convert with yImage"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\04_convert"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\04_convert\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExeName}"" --convert ""%1"""; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\05_bg_remove"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Remove background (yImage)"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\05_bg_remove"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\05_bg_remove\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExeName}"" --bg-remove ""%1"""; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\06_obj_remove"; ValueType: string; ValueName: "MUIVerb"; ValueData: "Remove object (yImage)"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\06_obj_remove"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\{#CtxKey}\shell\06_obj_remove\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExeName}"" --obj-remove ""%1"""; Tasks: ctxmenu

; Per-extension anchor pointing at the shared store.
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.png\Shell\yImage";  ValueType: string; ValueName: "MUIVerb"; ValueData: "yImage"; Flags: uninsdeletekey; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.png\Shell\yImage";  ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.png\Shell\yImage";  ValueType: string; ValueName: "ExtendedSubCommandsKey"; ValueData: "{#CtxKey}"; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.jpg\Shell\yImage";  ValueType: string; ValueName: "MUIVerb"; ValueData: "yImage"; Flags: uninsdeletekey; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.jpg\Shell\yImage";  ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.jpg\Shell\yImage";  ValueType: string; ValueName: "ExtendedSubCommandsKey"; ValueData: "{#CtxKey}"; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.jpeg\Shell\yImage"; ValueType: string; ValueName: "MUIVerb"; ValueData: "yImage"; Flags: uninsdeletekey; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.jpeg\Shell\yImage"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.jpeg\Shell\yImage"; ValueType: string; ValueName: "ExtendedSubCommandsKey"; ValueData: "{#CtxKey}"; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.webp\Shell\yImage"; ValueType: string; ValueName: "MUIVerb"; ValueData: "yImage"; Flags: uninsdeletekey; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.webp\Shell\yImage"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.webp\Shell\yImage"; ValueType: string; ValueName: "ExtendedSubCommandsKey"; ValueData: "{#CtxKey}"; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.bmp\Shell\yImage";  ValueType: string; ValueName: "MUIVerb"; ValueData: "yImage"; Flags: uninsdeletekey; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.bmp\Shell\yImage";  ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.bmp\Shell\yImage";  ValueType: string; ValueName: "ExtendedSubCommandsKey"; ValueData: "{#CtxKey}"; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.gif\Shell\yImage";  ValueType: string; ValueName: "MUIVerb"; ValueData: "yImage"; Flags: uninsdeletekey; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.gif\Shell\yImage";  ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.gif\Shell\yImage";  ValueType: string; ValueName: "ExtendedSubCommandsKey"; ValueData: "{#CtxKey}"; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.tif\Shell\yImage";  ValueType: string; ValueName: "MUIVerb"; ValueData: "yImage"; Flags: uninsdeletekey; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.tif\Shell\yImage";  ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.tif\Shell\yImage";  ValueType: string; ValueName: "ExtendedSubCommandsKey"; ValueData: "{#CtxKey}"; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.tiff\Shell\yImage"; ValueType: string; ValueName: "MUIVerb"; ValueData: "yImage"; Flags: uninsdeletekey; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.tiff\Shell\yImage"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.tiff\Shell\yImage"; ValueType: string; ValueName: "ExtendedSubCommandsKey"; ValueData: "{#CtxKey}"; Tasks: ctxmenu

Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.avif\Shell\yImage"; ValueType: string; ValueName: "MUIVerb"; ValueData: "yImage"; Flags: uninsdeletekey; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.avif\Shell\yImage"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#AppExeName}"",0"; Tasks: ctxmenu
Root: HKLM; Subkey: "Software\Classes\SystemFileAssociations\.avif\Shell\yImage"; ValueType: string; ValueName: "ExtendedSubCommandsKey"; ValueData: "{#CtxKey}"; Tasks: ctxmenu

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent
