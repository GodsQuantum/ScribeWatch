Unicode true
!include "MUI2.nsh"
!ifndef APP_NAME
  !error "Missing APP_NAME"
!endif
!ifndef APP_VERSION
  !error "Missing APP_VERSION"
!endif
!ifndef STAGING
  !error "Missing STAGING"
!endif
!ifndef OUTPUT
  !error "Missing OUTPUT"
!endif
!ifdef APP_ICON
Icon "${APP_ICON}"
UninstallIcon "${APP_ICON}"
!endif
Name "${APP_NAME} ${APP_VERSION}"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\${APP_NAME}"
InstallDirRegKey HKCU "Software\GodsQuantum\${APP_NAME}" "InstallPath"
RequestExecutionLevel user
!ifdef ARCHIVE_PAYLOAD
  ; Full bundle exceeds stock NSIS 2 GiB mmap limit for raw input.
  ; Embed a compressed payload inside one exe, and use built-in Windows tar.
  SetCompressor zlib
  SetCompress off
!else
  SetCompressor /SOLID lzma
!endif
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install" Main
 SetShellVarContext current
 SetOutPath "$INSTDIR"
!ifdef ARCHIVE_PAYLOAD
  File /oname=_ScribeWatch_payload.zip "${ARCHIVE_PAYLOAD}"
  IfFileExists "$SYSDIR\tar.exe" 0 unpack_error
  DetailPrint "Extracting included ASR and LLM engines..."
  nsExec::ExecToLog '"$SYSDIR\tar.exe" -xf "$INSTDIR\_ScribeWatch_payload.zip" -C "$INSTDIR"'
  Pop $0
  StrCmp $0 "0" unpack_success
unpack_error:
  DetailPrint "Unable to unpack standalone files."
  Delete "$INSTDIR\_ScribeWatch_payload.zip"
  Abort "ScribeWatch standalone extraction failed."
unpack_success:
  Delete "$INSTDIR\_ScribeWatch_payload.zip"
!else
  File /r "${STAGING}\*.*"
!endif
 WriteUninstaller "$INSTDIR\Uninstall.exe"
 CreateDirectory "$SMPROGRAMS\${APP_NAME}"
 CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_NAME}.exe"
 CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
 WriteRegStr HKCU "Software\GodsQuantum\${APP_NAME}" "InstallPath" "$INSTDIR"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\GodsQuantum-${APP_NAME}" "DisplayName" "${APP_NAME}"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\GodsQuantum-${APP_NAME}" "DisplayVersion" "${APP_VERSION}"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\GodsQuantum-${APP_NAME}" "DisplayIcon" "$INSTDIR\${APP_NAME}.exe,0"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\GodsQuantum-${APP_NAME}" "Publisher" "GodsQuantum"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\GodsQuantum-${APP_NAME}" "InstallLocation" "$INSTDIR"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\GodsQuantum-${APP_NAME}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
SectionEnd
Section "Uninstall"
 SetShellVarContext current
 Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
 Delete "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk"
 RMDir "$SMPROGRAMS\${APP_NAME}"
 RMDir /r "$INSTDIR\bin"
 RMDir /r "$INSTDIR\frontend"
 Delete "$INSTDIR\manifest-sha256.json"
 Delete "$INSTDIR\${APP_NAME}.exe"
 Delete "$INSTDIR\Uninstall.exe"
 RMDir "$INSTDIR"
 DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\GodsQuantum-${APP_NAME}"
 DeleteRegKey HKCU "Software\GodsQuantum\${APP_NAME}"
 ; User data stays under LOCALAPPDATA\AppName; never remove automatically.
SectionEnd
