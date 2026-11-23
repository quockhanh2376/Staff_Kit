; Staff Kit installer hooks (Tauri NSIS).
; Close a running Staff Kit before files are replaced, so updates need no OK click.
; Only the program is closed; the database in %LOCALAPPDATA%\io.staffkit.app is never touched here.
!macro NSIS_HOOK_PREINSTALL
  !if "${INSTALLMODE}" == "currentUser"
    nsis_tauri_utils::KillProcessCurrentUser "${MAINBINARYNAME}.exe"
  !else
    nsis_tauri_utils::KillProcess "${MAINBINARYNAME}.exe"
  !endif
  Pop $R0
  ; Give Windows a moment to release file and database handles.
  Sleep 1500
!macroend
