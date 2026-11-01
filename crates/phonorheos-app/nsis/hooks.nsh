; SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
; SPDX-License-Identifier: AGPL-3.0-or-later
;
; The installer puts the command's folder (bin\) on the system path and the uninstaller
; takes it off. PowerShell edits the registry value whole, keeping its type and the
; variables it names (%SystemRoot%), since NSIS's own strings are limited in length and
; a long path would be cut.
!include WinMessages.nsh

!macro PHONORHEOS_PATH op
  nsExec::ExecToLog `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$d = '$INSTDIR\bin'; $$k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey('SYSTEM\CurrentControlSet\Control\Session Manager\Environment', $$true); $$p = $$k.GetValue('Path', '', 'DoNotExpandEnvironmentNames'); $$parts = @($$p -split ';' | Where-Object { $$_ -and $$_ -ne $$d }); if ('${op}' -eq 'add') { $$parts += $$d }; $$k.SetValue('Path', ($$parts -join ';'), 'ExpandString')"`
  Pop $0
  ; Running programs, Explorer among them, read the new path from this notice.
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro PHONORHEOS_PATH add
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro PHONORHEOS_PATH remove
!macroend
