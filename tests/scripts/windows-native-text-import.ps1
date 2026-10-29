#Requires -Version 7.2
param(
 [Parameter(Mandatory)][string]$BinaryDirectory,
 [Parameter(Mandatory)][string]$KernelDirectory,
 [Parameter(Mandatory)][string]$OutputDirectory,
 [Parameter(Mandatory)][string]$TemporaryDirectory,
 [string]$CoordinatesFile=''
)
$ErrorActionPreference='Stop'
if(-not$IsWindows){throw 'Windows is required'}
foreach($name in @('BinaryDirectory','KernelDirectory','OutputDirectory','TemporaryDirectory')){
 Set-Variable -Name $name -Value (Resolve-Path -LiteralPath (Get-Variable -Name $name -ValueOnly)).ProviderPath
}
. "$PSScriptRoot/windows-native-log.ps1"
. "$PSScriptRoot/windows-native-office-content.ps1"
Add-Type -AssemblyName System.Drawing
Add-Type -Path "$PSScriptRoot/windows-native-ui.cs"
Add-Type -Path "$PSScriptRoot/windows-trial-job.cs"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ElyTextImportMemory {
 [StructLayout(LayoutKind.Sequential)] sealed class Memory {
  public uint length=(uint)Marshal.SizeOf<Memory>(),load;
  public ulong physical,availablePhysical,commit,availableCommit,virtualBytes,availableVirtual,extended;
 }
 [DllImport("kernel32.dll")] static extern bool GlobalMemoryStatusEx([In,Out]Memory m);
 public static ulong[] Available(){var m=new Memory();if(!GlobalMemoryStatusEx(m))throw new Exception("Memory snapshot failed");return new[]{m.availableCommit,m.availablePhysical};}
}
'@
$case=Join-Path $OutputDirectory ('native-text-import-'+[guid]::NewGuid().ToString('N').Substring(0,8))
New-Item -ItemType Directory -Path $case|Out-Null
$source=Join-Path $case 'source.csv'
$csv='00123;12345678901234567890;1.234567890123456789E+19;2026-10-28;0.12345678901234567890;=1+2;-0;中文😀;"  padded  "'+"`r`n"
[IO.File]::WriteAllText($source,$csv,[Text.UTF8Encoding]::new($false))
$sourceHash=(Get-FileHash -LiteralPath $source).Hash
$binary=Join-Path $BinaryDirectory 'ely-table.exe'
$worker=Join-Path $BinaryDirectory 'ely-lok-worker.exe'
$profile=Join-Path $case 'profile'
$log=Join-Path $profile 'ely-table/ely.log'
$preset=if($CoordinatesFile){ConvertFrom-Json ([IO.File]::ReadAllText((Resolve-Path -LiteralPath $CoordinatesFile).ProviderPath))}else{$null}
$r=[ordered]@{completed=$false;stage='start';forced_cleanup=$false;coordinates=@{};peak_workers=0;minimum_commit_mib=[long]::MaxValue;minimum_physical_mib=[long]::MaxValue;system_ime_tested=$false;binary_sha256=(Get-FileHash -LiteralPath $binary).Hash;worker_sha256=(Get-FileHash -LiteralPath $worker).Hash;harness_sha256=(Get-FileHash -LiteralPath $PSCommandPath).Hash;log_reader_sha256=(Get-FileHash -LiteralPath "$PSScriptRoot/windows-native-log.ps1").Hash;source_sha256=$sourceHash}
$job=$null;$app=$null;$handle=[IntPtr]0
function Log-Text {Read-ElyNativeLog $log}
function Check-Resources {
 $memory=[ElyTextImportMemory]::Available()
 $r.minimum_commit_mib=[Math]::Min($r.minimum_commit_mib,[long]($memory[0]/1MB))
 $r.minimum_physical_mib=[Math]::Min($r.minimum_physical_mib,[long]($memory[1]/1MB))
 $workers=0
 foreach($id in $job.ProcessIds){try{if([Diagnostics.Process]::GetProcessById($id).ProcessName -eq 'ely-lok-worker'){$workers++}}catch [ArgumentException]{}}
 $r.peak_workers=[Math]::Max($r.peak_workers,$workers)
 if($workers -gt 1 -or $memory[0] -lt 2GB -or $memory[1] -lt 1GB){throw 'Owned text trial resource guard'}
}
function Wait-Until([scriptblock]$Check,[int]$Seconds=30) {
 $deadline=[DateTime]::UtcNow.AddSeconds($Seconds)
 while([DateTime]::UtcNow -lt $deadline){
  Check-Resources
  if(&$Check){return}
  if($app.HasExited){throw "Owned app exited at $($r.stage)"}
  Start-Sleep -Milliseconds 100
 }
 throw "Timed out at $($r.stage)"
}
function Capture([string]$Name) {
 [ElyNativeUI]::Frame($handle)
 $rect=[ElyNativeUI]::PhysicalBounds($handle)
 $bitmap=[Drawing.Bitmap]::new($rect.R-$rect.L,$rect.B-$rect.T)
 $graphics=[Drawing.Graphics]::FromImage($bitmap)
 try{
  $dc=$graphics.GetHdc()
  try{$ok=[ElyNativeUI]::PrintWindow($handle,$dc,2)}finally{$graphics.ReleaseHdc($dc)}
  if(-not$ok){throw 'Owned capture failed'}
  $bitmap.Save((Join-Path $case "$Name.png"))
 }finally{$graphics.Dispose();$bitmap.Dispose()}
}
function Click-Point([string]$Name) {
 $point=if($preset){$preset.$Name}else{
  $file=Join-Path $case "$Name.json"
  Write-Output "Await coordinate file: $file" | Out-Host
  Wait-Until {Test-Path -LiteralPath $file -PathType Leaf} 180
  ConvertFrom-Json ([IO.File]::ReadAllText($file))
 }
 if($null -eq $point.x -or $null -eq $point.y){throw 'Coordinate pair missing'}
 $x=[int]$point.x;$y=[int]$point.y
 $bounds=[ElyNativeUI]::PhysicalBounds($handle)
 if($x -lt 1 -or $y -lt 1 -or $x -ge $bounds.R-$bounds.L -or $y -ge $bounds.B-$bounds.T){throw 'Coordinate outside owned image'}
 $offset=[regex]::Match($r.geometry,'offset=(-?\d+),(-?\d+)')
 if(-not$offset.Success){throw 'Owned client origin missing'}
 $r.coordinates[$Name]=@{x=$x;y=$y}
 [ElyNativeUI]::Click($handle,$x-[int]$offset.Groups[1].Value,$y-[int]$offset.Groups[2].Value)
}
try {
 $info=[Diagnostics.ProcessStartInfo]::new($binary)
 $info.UseShellExecute=$false;$info.CreateNoWindow=$true
 $info.ArgumentList.Add($source)
 foreach($pair in @{ELY_DATA_DIR=$profile;ELY_LOK_DIR=$KernelDirectory;ELY_LOK_WORKER=$worker;ELY_SMOKE='hold';ELY_NO_DIALOG='1';ELY_LOCALE='en';TEMP=$TemporaryDirectory;TMP=$TemporaryDirectory}.GetEnumerator()){$info.Environment[$pair.Key]=$pair.Value}
 $job=[ElyTrialJob]::Start($info);$app=$job.Root;$r.pid=$app.Id
 $r.stage='preview'
 Wait-Until {$app.Refresh();$script:handle=$app.MainWindowHandle;$handle -ne [IntPtr]0 -and (Log-Text) -match 'import scan: 9 columns, broken false'} 45
 $r.window_dpi=[ElyNativeUI]::GetDpiForWindow($handle)
 if($r.window_dpi -ne 168){throw 'This coordinate trial requires 168 DPI'}
 $r.geometry=[ElyNativeUI]::Geometry($handle)
 Capture 'preview'
 Write-Output "Owned trial folder: $case" | Out-Host
 $r.stage='choose-text-mode'
 Click-Point 'switch'
 Wait-Until {(Log-Text) -match 'switch .*table-import-text-only.*: on'}
 $r.text_mode_selected=$true
 Capture 'text-mode'
 $r.stage='import'
 Click-Point 'open'
 Wait-Until {(Log-Text) -match 'table: imports as .*text only true' -and (Test-ElyOpenedInput (Log-Text) $source)} 45
 $r.imported_text_mode=$true
 Capture 'imported'
 $r.stage='save-as'
 [ElyNativeUI]::Click($handle,163,87);[ElyNativeUI]::Frame($handle)
 [ElyNativeUI]::Click($handle,205,293)
 $script:panel=[IntPtr]0
 Wait-Until {$script:panel=[ElyNativeUI]::Dialog([uint32]$app.Id,1001,'Edit');$panel -ne [IntPtr]0}
 $saved=Join-Path $case 'saved.xlsx'
 [ElyNativeUI]::Text([ElyNativeUI]::Child($panel,1001,'Edit'),$saved)
 [ElyNativeUI]::Command($panel,1)
 Wait-Until {(Log-Text) -match 'office: saved ' -and (Test-Path -LiteralPath $saved)}
 $r.saved_path=$saved;$r.saved_sha256=(Get-FileHash -LiteralPath $saved).Hash
 Capture 'saved'
 $r.stage='normal-close'
 [ElyNativeUI]::Close($handle)
 if(-not$app.WaitForExit(15000)){throw 'Owned text app did not close normally'}
 $deadline=[DateTime]::UtcNow.AddSeconds(3)
 while($job.Active -gt 0 -and [DateTime]::UtcNow -lt $deadline){Start-Sleep -Milliseconds 50}
 if($job.Active -gt 0){throw 'Owned text job left descendants'}
 $r.job_empty=$true;$r.exit_code=$app.ExitCode
 if(-not[Threading.Tasks.Task]::WaitAll([Threading.Tasks.Task[]]@($job.Stdout,$job.Stderr),5000)){throw 'Owned text output did not close'}
 $job.Stdout.GetAwaiter().GetResult()|Set-Content -LiteralPath "$case/stdout.log" -Encoding utf8
 $stderr=$job.Stderr.GetAwaiter().GetResult()
 $stderr|Set-Content -LiteralPath "$case/stderr.log" -Encoding utf8
 $r.error_lines=@([regex]::Matches($stderr,'(?m)^.*\bERROR\b[^\r\n]*')|ForEach-Object Value)
 $r.invalid_handle_count=[regex]::Matches($stderr,'(?i)0x80070578').Count
 if($r.exit_code -ne 0 -or $r.error_lines.Count -or $r.invalid_handle_count){throw 'Owned text exit/log failed'}
 if((Get-FileHash -LiteralPath $source).Hash -ne $sourceHash){throw 'Import changed original CSV'}
 $r.original_unchanged=$true;$r.completed=$true
} catch {
 $r.error=$_.Exception.Message
 if($app -and -not$app.HasExited -and $handle -ne [IntPtr]0){try{Capture 'failure'}catch{$r.capture_error=$_.Exception.Message}}
} finally {
 if($job){if($job.Active -gt 0){$job.Stop();$r.forced_cleanup=$true};$job.Dispose()}
 $r.coordinates|ConvertTo-Json|Set-Content -LiteralPath "$case/coordinates.json" -Encoding utf8
 $r|ConvertTo-Json -Depth 6|Set-Content -LiteralPath "$case/results.json" -Encoding utf8
 Write-Output "$case/results.json"
}
if(-not$r.completed -or $r.forced_cleanup){exit 1}
