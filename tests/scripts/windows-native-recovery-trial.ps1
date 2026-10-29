#Requires -Version 7.2
param(
 [Parameter(Mandatory)][string]$BinaryDirectory,
 [Parameter(Mandatory)][string]$KernelDirectory,
 [Parameter(Mandatory)][string]$OutputDirectory,
 [Parameter(Mandatory)][string]$TemporaryDirectory
)
$ErrorActionPreference='Stop'
if(-not$IsWindows){throw 'Windows is required'}
foreach($name in @('BinaryDirectory','KernelDirectory','OutputDirectory','TemporaryDirectory')){
 Set-Variable -Name $name -Value (Resolve-Path -LiteralPath (Get-Variable -Name $name -ValueOnly)).ProviderPath
}
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type -Path "$PSScriptRoot/windows-native-ui.cs"
Add-Type -Path "$PSScriptRoot/windows-trial-job.cs"
. "$PSScriptRoot/windows-native-office-content.ps1"
. "$PSScriptRoot/windows-native-log.ps1"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ElyRecoveryMemory {
 [StructLayout(LayoutKind.Sequential)] sealed class Memory {
  public uint length=(uint)Marshal.SizeOf<Memory>(),load;
  public ulong physical,availablePhysical,commit,availableCommit,virtualBytes,availableVirtual,extended;
 }
 [DllImport("kernel32.dll")] static extern bool GlobalMemoryStatusEx([In,Out]Memory m);
 public static ulong[] Available(){var m=new Memory();if(!GlobalMemoryStatusEx(m))throw new Exception("Memory snapshot failed");return new[]{m.availableCommit,m.availablePhysical};}
}
'@
$case=Join-Path $OutputDirectory ('native-recovery-'+[guid]::NewGuid().ToString('N').Substring(0,8))
New-Item -ItemType Directory -Path $case|Out-Null
$repo=Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$binary=Join-Path $BinaryDirectory 'ely-table.exe'
$worker=Join-Path $BinaryDirectory 'ely-lok-worker.exe'
$source=Join-Path $case 'input.xlsx'
Copy-Item -LiteralPath (Join-Path $repo 'packaging/windows/new/new.xlsx') -Destination $source
$sourceHash=(Get-FileHash -LiteralPath $source).Hash
$caseProfile=Join-Path $case 'profile'
$log=Join-Path $caseProfile 'ely-table/ely.log'
$marker='Round20 recovery 中文 😀'
$job=$null;$app=$null;$handle=[IntPtr]0;$phase='first';$offset=0;$prefix=''
$r=[ordered]@{completed=$false;injected_crash=$false;unexpected_cleanup=$false;stage='start';system_ime_tested=$false;encrypted_native_tested=$false;peak_workers=0;minimum_commit_mib=[long]::MaxValue;minimum_physical_mib=[long]::MaxValue}
$r.binary_sha256=(Get-FileHash -LiteralPath $binary).Hash
$r.worker_sha256=(Get-FileHash -LiteralPath $worker).Hash
$r.harness_sha256=(Get-FileHash -LiteralPath $PSCommandPath).Hash
$r.reader_sha256=(Get-FileHash -LiteralPath "$PSScriptRoot/windows-native-office-content.ps1").Hash
$r.ui_helper_sha256=(Get-FileHash -LiteralPath "$PSScriptRoot/windows-native-ui.cs").Hash
$r.job_helper_sha256=(Get-FileHash -LiteralPath "$PSScriptRoot/windows-trial-job.cs").Hash
$r.log_reader_sha256=(Get-FileHash -LiteralPath "$PSScriptRoot/windows-native-log.ps1").Hash
$r.log_reads=@{reads=0;chunked_reads=0;max_chunks=0}

function Log-Text {
 $text=Read-ElyNativeLog $log $r.log_reads
 if($text.Length -lt $script:offset){throw 'Owned log was unexpectedly truncated'}
 if(-not$text.StartsWith($script:prefix,[StringComparison]::Ordinal)){throw 'Owned log prefix changed'}
 return $text.Substring($script:offset)
}
function Check-Resources {
 $memory=[ElyRecoveryMemory]::Available()
 $r.minimum_commit_mib=[math]::Min($r.minimum_commit_mib,[long]($memory[0]/1MB))
 $r.minimum_physical_mib=[math]::Min($r.minimum_physical_mib,[long]($memory[1]/1MB))
 $workers=0
 foreach($id in $job.ProcessIds){try{if([Diagnostics.Process]::GetProcessById($id).ProcessName -eq 'ely-lok-worker'){$workers++}}catch [ArgumentException]{}}
 $r.peak_workers=[math]::Max($r.peak_workers,$workers)
 if($workers -gt 1 -or $memory[0] -lt 2GB -or $memory[1] -lt 1GB){throw 'Owned trial resource guard stopped the run'}
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
 $rect=[ElyNativeUI]::PhysicalBounds($handle)
 $bitmap=[Drawing.Bitmap]::new($rect.R-$rect.L,$rect.B-$rect.T)
 $graphics=[Drawing.Graphics]::FromImage($bitmap)
 try{
  $dc=$graphics.GetHdc()
  try{$ok=[ElyNativeUI]::PrintWindow($handle,$dc,2)}finally{$graphics.ReleaseHdc($dc)}
  if(-not$ok){throw 'Owned window capture failed'}
  $bitmap.Save((Join-Path $case "$Name.png"))
 }finally{$graphics.Dispose();$bitmap.Dispose()}
}
function Start-App([string]$Phase) {
 $script:phase=$Phase
 $script:prefix=Read-ElyNativeLog $log $r.log_reads
 $script:offset=$prefix.Length
 if($script:offset -gt 1MB){throw 'This isolated trial does not permit log trimming'}
 $info=[Diagnostics.ProcessStartInfo]::new($binary)
 $info.UseShellExecute=$false;$info.CreateNoWindow=$true
 $info.ArgumentList.Add($source)
 foreach($pair in @{ELY_DATA_DIR=$caseProfile;ELY_LOK_DIR=$KernelDirectory;ELY_LOK_WORKER=$worker;ELY_SMOKE='hold';ELY_NO_DIALOG='1';ELY_LOCALE='en';TEMP=$TemporaryDirectory;TMP=$TemporaryDirectory}.GetEnumerator()){$info.Environment[$pair.Key]=$pair.Value}
 $script:job=[ElyTrialJob]::Start($info)
 $script:app=$job.Root;$script:handle=[IntPtr]0
 $r["${Phase}_pid"]=$app.Id
 Wait-Until {$app.Refresh();$script:handle=$app.MainWindowHandle;$handle -ne [IntPtr]0 -and (Test-ElyOpenedInput (Log-Text) $source)} 45
 if([ElyNativeUI]::GetDpiForWindow($handle) -ne 168){throw 'This coordinate trial requires the validated 168 DPI layout'}
 $r.window_dpi=168
 [ElyNativeUI]::Frame($handle)
}
function Drain {
 if(-not$app.WaitForExit(5000)){throw 'Owned process did not exit'}
 $deadline=[DateTime]::UtcNow.AddSeconds(3)
 while($job.Active -gt 0 -and [DateTime]::UtcNow -lt $deadline){Start-Sleep -Milliseconds 100}
 if($job.Active -gt 0){throw 'Owned job left descendants'}
 if(-not[Threading.Tasks.Task]::WaitAll([Threading.Tasks.Task[]]@($job.Stdout,$job.Stderr),5000)){throw 'Owned output did not close'}
 $job.Stdout.GetAwaiter().GetResult()|Set-Content -LiteralPath "$case/$phase-stdout.log" -Encoding utf8
 $stderr=$job.Stderr.GetAwaiter().GetResult()
 $stderr|Set-Content -LiteralPath "$case/$phase-stderr.log" -Encoding utf8
 $r["${phase}_exit_code"]=$app.ExitCode
 $r["${phase}_job_empty"]=$true
 $r["${phase}_error_lines"]=@([regex]::Matches($stderr,'(?m)^.*\bERROR\b[^\r\n]*')|ForEach-Object Value)
 $job.Dispose();$script:job=$null;$script:app=$null
}
function Type-Text([string]$Text) {
 foreach($character in $Text.ToCharArray()){
  if(-not[ElyNativeUI]::PostMessageW($handle,0x102,[IntPtr][int]$character,[IntPtr]0)){throw 'Owned character post failed'}
 }
}
function Write-Cell([string]$Address,[string]$Text) {
 $before=[regex]::Matches((Log-Text),'ran cell.goto').Count
 [ElyNativeUI]::Click($handle,80,193);[ElyNativeUI]::Frame($handle)
 [ElyNativeUI]::Key($handle,35)
 for($i=0;$i -lt 40;$i++){[ElyNativeUI]::Key($handle,8)}
 Type-Text $Address;[ElyNativeUI]::Key($handle,13)
 Wait-Until {([regex]::Matches((Log-Text),'ran cell.goto').Count) -gt $before}
 [ElyNativeUI]::Frame($handle)
 $before=[regex]::Matches((Log-Text),'ran cell.enter').Count
 [ElyNativeUI]::Click($handle,400,193);[ElyNativeUI]::Frame($handle)
 [ElyNativeUI]::Key($handle,35)
 for($i=0;$i -lt 40;$i++){[ElyNativeUI]::Key($handle,8)}
 Type-Text $Text;[ElyNativeUI]::Key($handle,13)
 Wait-Until {([regex]::Matches((Log-Text),'ran cell.enter').Count) -gt $before}
 [ElyNativeUI]::Frame($handle)
}
try{
 Start-App 'first'
 $r.stage='edit'
 Write-Cell 'A1' $marker
 Write-Cell 'B1' '=1+2'
 $r.stage='wait-complete-point';$script:point=$null
 Wait-Until {
  $root=Join-Path $caseProfile 'ely-table/recovery'
  if(-not(Test-Path -LiteralPath $root)){return $false}
  foreach($candidate in Get-ChildItem -LiteralPath $root -Recurse -File -Filter '*.bin'|Sort-Object LastWriteTimeUtc -Descending){
   try{Assert-ElyOfficeContent $candidate.FullName 'table' $marker '' $false $false;$script:point=$candidate.FullName;return $true}
   catch{$r.last_point_read_error=$_.Exception.Message}
  }
  return $false
 } 45
 Copy-Item -LiteralPath $point -Destination "$case/committed-point.bin"
 $r.point_sha256=(Get-FileHash -LiteralPath "$case/committed-point.bin").Hash
 $r.complete_point_before_crash=$true
 $r.pre_injection_error_lines=@([regex]::Matches((Log-Text),'(?m)^.*\bERROR\b[^\r\n]*')|ForEach-Object Value)
 if($r.pre_injection_error_lines.Count){throw 'App reported an error before the injected crash'}
 Capture 'before-crash'
 $r.stage='injected-crash'
 $job.Stop();$r.injected_crash=$true
 Drain
 if($r.first_exit_code -eq 0){throw 'The injected termination unexpectedly reported a clean exit'}
 if((Get-FileHash -LiteralPath $source).Hash -ne $sourceHash){throw 'Crash changed original input'}
 $r.stage='restore'
 Start-App 'reopen'
 Wait-Until {
  $text=Log-Text
  $restore=[regex]::Match($text,'recovery: reopening a xlsx')
  if(-not$restore.Success){return $false}
  $opens=@([regex]::Matches($text,'opened .+ as tab \d+, risk '))
  $after=@($opens|Where-Object {$_.Index -gt $restore.Index})
  return $after.Count -gt 0 -and $text.LastIndexOf('first tile drawn') -gt $after[-1].Index
 } 45
 $r.crash_copy_opened=$true
 Capture 'restored'
 $r.stage='save-recovered-copy'
 [ElyNativeUI]::Click($handle,163,87);[ElyNativeUI]::Frame($handle)
 [ElyNativeUI]::Click($handle,205,293)
 $script:panel=[IntPtr]0
 Wait-Until {$script:panel=[ElyNativeUI]::Dialog([uint32]$app.Id,1001,'Edit');$panel -ne [IntPtr]0}
 $saved=Join-Path $case 'recovered-copy.xlsx'
 [ElyNativeUI]::Text([ElyNativeUI]::Child($panel,1001,'Edit'),$saved)
 [ElyNativeUI]::Command($panel,1)
 Wait-Until {(Log-Text) -match 'office: saved ' -and (Test-Path -LiteralPath $saved)}
 Assert-ElyOfficeContent $saved 'table' $marker '' $false $false
 $r.recovered_contents_verified=$true
 $r.saved_sha256=(Get-FileHash -LiteralPath $saved).Hash
 $r.stage='normal-close'
 [ElyNativeUI]::Close($handle)
 if(-not$app.WaitForExit(15000)){throw 'Recovered app did not close normally'}
 Drain
 if($r.reopen_exit_code -ne 0 -or $r.reopen_error_lines.Count){throw 'Recovered app exit/log failed'}
 Assert-ElyOfficeContent $saved 'table' $marker '' $false $false
 if((Get-FileHash -LiteralPath $source).Hash -ne $sourceHash){throw 'Recovery overwrote original input'}
 $r.original_unchanged=$true
 $r.completed=$true
}catch{
 $r.error=$_.Exception.Message
 if($app -and -not$app.HasExited){try{Capture 'failure'}catch{$r.capture_error=$_.Exception.Message}}
}finally{
 if($job){
  if($job.Active -gt 0){$job.Stop();$r.unexpected_cleanup=$true}
  try{Drain}catch{$r.cleanup_error=$_.Exception.Message;if($job){$job.Dispose()}}
 }
 $r|ConvertTo-Json -Depth 5|Set-Content -LiteralPath "$case/results.json" -Encoding utf8
 Write-Output "$case/results.json"
}
if(-not$r.completed -or $r.unexpected_cleanup -or $r.cleanup_error){exit 1}
