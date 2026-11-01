; SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
; SPDX-License-Identifier: AGPL-3.0-or-later
;
; The installer puts the command's folder (bin\) on the system path and the uninstaller
; takes it off. PowerShell edits the registry value whole, keeping its type and the
; variables it names (%SystemRoot%), since NSIS's own strings are limited in length and
; a long path would be cut. The folder reaches PowerShell in an environment variable,
; so that no character in it can break the script.
!include WinMessages.nsh

!macro PHONORHEOS_PATH op
  System::Call 'Kernel32::SetEnvironmentVariable(t "PHONORHEOS_BIN", t "$INSTDIR\bin")'
  nsExec::ExecToLog `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$d = $$env:PHONORHEOS_BIN; $$k = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey('SYSTEM\CurrentControlSet\Control\Session Manager\Environment', $$true); $$p = $$k.GetValue('Path', '', 'DoNotExpandEnvironmentNames'); $$parts = @($$p -split ';' | Where-Object { $$_ -and $$_ -ne $$d }); if ('${op}' -eq 'add') { $$parts += $$d }; $$k.SetValue('Path', ($$parts -join ';'), 'ExpandString')"`
  Pop $0
  StrCmp $0 "0" +2
    MessageBox MB_ICONEXCLAMATION|MB_OK "The system path could not be changed (${op} $INSTDIR\bin): $0" /SD IDOK
  ; Running programs, Explorer among them, read the new path from this notice.
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro PHONORHEOS_PATH add
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro PHONORHEOS_PATH remove
!macroend
