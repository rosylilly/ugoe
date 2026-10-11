; スタジオの仮想カメラ (T0807、docs/adr/0028-studio.md の 8.、docs/studio.md#windows-media-foundation)。
; Frame Server が読み込むメディアソースの DLL (ugoe_studio_vcam.dll) を入れ、管理者の権限で COM に登録する。
; DLL は `pnpm vcam` (scripts/build-vcam.mjs、tauri.windows.conf.json の beforeBuildCommand) が target/vcam に作る。
; インストーラは perMachine (管理者の権限) で動く。スタジオの実行中には権限を求めない。

!define UGOE_VCAM_DLL "ugoe_studio_vcam.dll"
; 入れる DLL の場所。`${__FILEDIR__}` は、マクロの中では差し込んだ先 (installer.nsi) の場所になるので、読み込んだときに決める。
; 区切りは `\` にする。Windows の makensis は File のパスを最後の `\` でディレクトリとファイル名に分けるので、`/` で
; つなぐと `windows/../..` までをファイル名として探し、見つからない (T0088)。Linux の makensis は `\` を `/` に直して読む。
!define UGOE_VCAM_SOURCE "${__FILEDIR__}\..\..\..\..\target\vcam\x86_64-pc-windows-msvc\release\${UGOE_VCAM_DLL}"

!macro NSIS_HOOK_PREINSTALL
  ; 入れ直しのとき、前の DLL は Frame Server が読み込んだままのことがある。使用中のファイルは上書きできないが、
  ; 名前は変えられるので退け、再起動のときに消す。
  IfFileExists "$INSTDIR\${UGOE_VCAM_DLL}" 0 +4
    Delete "$INSTDIR\${UGOE_VCAM_DLL}.old"
    Rename "$INSTDIR\${UGOE_VCAM_DLL}" "$INSTDIR\${UGOE_VCAM_DLL}.old"
    Delete /REBOOTOK "$INSTDIR\${UGOE_VCAM_DLL}.old"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  SetOutPath "$INSTDIR"
  File "${UGOE_VCAM_SOURCE}"
  ; 64 ビットの regsvr32 で登録する (HKLM\SOFTWARE\Classes\CLSID に書く。DllRegisterServer)。
  ${DisableX64FSRedirection}
  ExecWait '"$SYSDIR\regsvr32.exe" /s "$INSTDIR\${UGOE_VCAM_DLL}"' $0
  ${EnableX64FSRedirection}
  ${If} $0 != 0
    DetailPrint "regsvr32 ${UGOE_VCAM_DLL} failed ($0); the virtual camera will not be available"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${DisableX64FSRedirection}
  ExecWait '"$SYSDIR\regsvr32.exe" /s /u "$INSTDIR\${UGOE_VCAM_DLL}"'
  ${EnableX64FSRedirection}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Frame Server が読み込んだままなら、再起動のときに消す。
  Delete /REBOOTOK "$INSTDIR\${UGOE_VCAM_DLL}"
  Delete /REBOOTOK "$INSTDIR\${UGOE_VCAM_DLL}.old"
!macroend
