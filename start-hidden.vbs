Set sh = CreateObject("WScript.Shell")
sh.Run "powershell -WindowStyle Hidden -ExecutionPolicy Bypass -File """ & CreateObject("Scripting.FileSystemObject").GetParentFolderName(WScript.ScriptFullName) & "\razer-battery.ps1""", 0, False
