# MSI installer

`dist\OpenDownloadManager-1.0.7.msi` installs Open Download Manager for the
current user without elevation.

## What it installs

| Item | Location |
|---|---|
| `OpenDownloadManager.exe` | `%LOCALAPPDATA%\Programs\Open Download Manager` |
| `odm-native-host.exe` | same folder |
| `odm.ico` | same folder |
| Browser extension | `...\Open Download Manager\extension` |
| Start-menu shortcut | `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Open Download Manager` |
| Add/Remove Programs entry | `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\{ProductCode}` |
| Native messaging manifests | `%APPDATA%\OpenDownloadManager\native` (written at install time with the real path) |
| Chrome/Edge/CocCoc/Firefox host keys | `HKCU\Software\...\NativeMessagingHosts\com.opendm.host` |

The MSI is a per-user package (`ALLUSERS` per-user, `InstallPrivileges=limited`).
On systems where Windows Installer still creates a machine-context product
registration, the machine ARP entry is hidden (`ARPSYSTEMCOMPONENT=1`) and a
per-user ARP entry is installed by a component, so Control Panel shows one
clean entry with a working `MsiExec.exe /x {ProductCode}` uninstall command.

## Install / uninstall

```powershell
msiexec /i .\dist\OpenDownloadManager-1.0.7.msi
msiexec /x .\dist\OpenDownloadManager-1.0.7.msi
```

Silent:

```powershell
msiexec /i .\dist\OpenDownloadManager-1.0.7.msi /qn /norestart /l*v install.log
```

Uninstall removes the program files, shortcut, ARP entry, native-messaging
manifests and browser host registry keys. It leaves `%APPDATA%\OpenDownloadManager`
(download list, settings, temp parts) in place so upgrades and reinstall keep
state; delete that folder manually for a full reset.

## Building

The build uses WiX 3.14 (already extracted in `tools\wix`):

```powershell
.\build.ps1                              # app + extension into dist
.\installer\build-msi.ps1 -Version 1.0.7 # produces dist\OpenDownloadManager-1.0.7.msi
```

`installer\Product.wxs` contains the product definition, `license.rtf` the
license page, and `banner.bmp` / `dialog.bmp` the WiX UI artwork. Stable
component/upgrade GUIDs live in `installer\guids.txt`. The `UpgradeCode` must
never change; `ProductCode` is generated per build and `MajorUpgrade` handles
version transitions.

The package is currently unsigned. Sign it with `signtool` and your
Authenticode certificate before public distribution to avoid SmartScreen
warnings.
