; EasyMD NSIS 安装钩子：注册 Windows 右键菜单"用 EasyMD 打开"（HKCU 当前用户，无需管理员）
; 卸载时自动清理

!macro NSIS_HOOK_POSTINSTALL
  ; 右键菜单直达项
  WriteRegStr HKCU "Software\Classes\EasyMD.file" "" "EasyMD 文档"
  WriteRegStr HKCU "Software\Classes\EasyMD.file\DefaultIcon" "" "$INSTDIR\EasyMD.exe,0"
  WriteRegStr HKCU "Software\Classes\EasyMD.file\shell\open\command" "" '"$INSTDIR\EasyMD.exe" "%1"'

  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.md\shell\EasyMD" "" "用 EasyMD 打开"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.md\shell\EasyMD" "Icon" "$INSTDIR\EasyMD.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.md\shell\EasyMD\command" "" '"$INSTDIR\EasyMD.exe" "%1"'
  WriteRegStr HKCU "Software\Classes\.md\OpenWithProgids\EasyMD.file" "" ""

  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.markdown\shell\EasyMD" "" "用 EasyMD 打开"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.markdown\shell\EasyMD" "Icon" "$INSTDIR\EasyMD.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.markdown\shell\EasyMD\command" "" '"$INSTDIR\EasyMD.exe" "%1"'

  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.canvas\shell\EasyMD" "" "用 EasyMD 打开"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.canvas\shell\EasyMD" "Icon" "$INSTDIR\EasyMD.exe"
  WriteRegStr HKCU "Software\Classes\SystemFileAssociations\.canvas\shell\EasyMD\command" "" '"$INSTDIR\EasyMD.exe" "%1"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.md\shell\EasyMD"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.markdown\shell\EasyMD"
  DeleteRegKey HKCU "Software\Classes\SystemFileAssociations\.canvas\shell\EasyMD"
  DeleteRegValue HKCU "Software\Classes\.md\OpenWithProgids" "EasyMD.file"
  DeleteRegKey HKCU "Software\Classes\EasyMD.file"
!macroend
