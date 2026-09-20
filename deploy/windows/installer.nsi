; NodeX Windows NSIS Installer Script
; Pure Rust Native P2P E2EE Messenger

!define PRODUCT_NAME "NodeX"
!define PRODUCT_VERSION "1.0.0"
!define PRODUCT_PUBLISHER "NodeX Network"
!define PRODUCT_WEB_SITE "https://nodex.network"
!define PRODUCT_DIR_REGKEY "Software\Microsoft\Windows\CurrentVersion\App Paths\nodex.exe"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
!define PRODUCT_UNINST_ROOT_KEY "HKLM"

SetCompressor lzma

!include "MUI2.nsh"

!define MUI_ABORTWARNING
!define MUI_ICON "..\..\nodex-gui\assets\logo.ico"
!define MUI_UNICON "..\..\nodex-gui\assets\logo.ico"

; Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_INSTFILES

; Languages
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Russian"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "NodeX_Setup_${PRODUCT_VERSION}.exe"
InstallDir "$PROGRAMFILES64\NodeX"
InstallDirRegKey HKLM "${PRODUCT_DIR_REGKEY}" ""
ShowInstDetails show
ShowUnInstDetails show

Section "MainSection" SEC01
  SetOutPath "$INSTDIR"
  SetOverwrite ifnewer
  File "..\..\target\release\nodex.exe"
  File "..\..\nodex-gui\assets\logo.png"

  CreateDirectory "$SMPROGRAMS\NodeX"
  CreateShortCut "$SMPROGRAMS\NodeX\NodeX.lnk" "$INSTDIR\nodex.exe"
  CreateShortCut "$DESKTOP\NodeX.lnk" "$INSTDIR\nodex.exe"
SectionEnd

Section -Post
  WriteUninstaller "$INSTDIR\uninst.exe"
  WriteRegStr HKLM "${PRODUCT_DIR_REGKEY}" "" "$INSTDIR\nodex.exe"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayName" "$(^Name)"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "UninstallString" "$INSTDIR\uninst.exe"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayIcon" "$INSTDIR\nodex.exe"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
SectionEnd

Section Uninstall
  Delete "$INSTDIR\nodex.exe"
  Delete "$INSTDIR\logo.png"
  Delete "$INSTDIR\uninst.exe"

  Delete "$DESKTOP\NodeX.lnk"
  Delete "$SMPROGRAMS\NodeX\NodeX.lnk"
  RMDir "$SMPROGRAMS\NodeX"
  RMDir "$INSTDIR"

  DeleteRegKey ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}"
  DeleteRegKey HKLM "${PRODUCT_DIR_REGKEY}"
  SetAutoClose true
SectionEnd
