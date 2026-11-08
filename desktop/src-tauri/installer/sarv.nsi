Unicode true
ManifestDPIAware true

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"

!ifndef PRODUCT_VERSION
  !define PRODUCT_VERSION "0.2.0-rc.1"
!endif

; Per-user setup. It does not need an administrator account.
Name "Sarv"
OutFile "Sarv-${PRODUCT_VERSION}-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Sarv"
InstallDirRegKey HKCU "Software\Sarv" "InstallDir"
RequestExecutionLevel user
ShowInstDetails show
ShowUninstDetails show

!define MUI_ICON "icon.ico"
!define MUI_UNICON "icon.ico"
!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TITLE "Install Sarv"
!define MUI_WELCOMEPAGE_TEXT "Sarv keeps this shop's books on this computer.$\r$\n$\r$\nسرو دفتر این فروشگاه را روی همین رایانه نگه می‌دارد.$\r$\n$\r$\nThe books stay in your profile. Uninstalling Sarv does not delete them.$\r$\n$\r$\nThis setup is not code-signed. Windows may ask you to confirm it.$\r$\nMicrosoft Edge WebView2, already present on current Windows 10 and 11, is required to open the window."
!define MUI_FINISHPAGE_TITLE "Sarv is installed"
!define MUI_FINISHPAGE_TEXT "Open Sarv from the Start menu. The first screen asks for a shop password. There is no default password."
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_TEXT "Open Sarv"
!define MUI_FINISHPAGE_RUN_FUNCTION "LaunchSarv"
!define MUI_FINISHPAGE_LINK "Project page"
!define MUI_FINISHPAGE_LINK_LOCATION "https://github.com/Fearlex-creator/sarv-quote"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE.txt"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Function LaunchSarv
  Exec "$INSTDIR\Sarv.exe"
FunctionEnd

Section "Sarv"
  SetOutPath "$INSTDIR"
  File /r "payload\*.*"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateShortcut "$SMPROGRAMS\Sarv.lnk" "$INSTDIR\Sarv.exe" "" "$INSTDIR\icon.ico" 0

  WriteRegStr HKCU "Software\Sarv" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarv" "DisplayName" "Sarv"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarv" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarv" "Publisher" "Sarv"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarv" "DisplayIcon" "$INSTDIR\Sarv.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarv" "UninstallString" "$\"$INSTDIR\Uninstall.exe$\""
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarv" "Comments" "Shop accounting on this computer. Not a tax-filing service."
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarv" "URLInfoAbout" "https://github.com/Fearlex-creator/sarv-quote"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarv" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarv" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarv" "EstimatedSize" $0
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\Sarv.lnk"
  RMDir /r "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Sarv"
  DeleteRegKey HKCU "Software\Sarv"
SectionEnd
