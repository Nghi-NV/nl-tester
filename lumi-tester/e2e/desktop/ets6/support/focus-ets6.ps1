$ErrorActionPreference = 'Stop'

Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Ets6Focus {
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
'@

$process = Get-Process -Name 'ETS6N' -ErrorAction Stop |
    Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero } |
    Sort-Object StartTime -Descending |
    Select-Object -First 1
if (-not $process) { throw 'ETS6 main window was not found.' }

[void][Ets6Focus]::ShowWindowAsync($process.MainWindowHandle, 9)
$shell = New-Object -ComObject WScript.Shell
$shell.SendKeys('%')
Start-Sleep -Milliseconds 150
[void][Ets6Focus]::BringWindowToTop($process.MainWindowHandle)
[void][Ets6Focus]::SetForegroundWindow($process.MainWindowHandle)
Start-Sleep -Milliseconds 400

if ([Ets6Focus]::GetForegroundWindow() -ne $process.MainWindowHandle) {
    throw 'Could not bring the ETS6 window to the foreground.'
}
Write-Output 'ETS6 foreground assertion passed.'
