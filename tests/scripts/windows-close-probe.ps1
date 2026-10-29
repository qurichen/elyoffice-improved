#Requires -Version 7.2
param(
 [Parameter(Mandatory)][string]$BinaryDirectory,
 [Parameter(Mandatory)][string]$KernelDirectory,
 [Parameter(Mandatory)][string]$InputFile,
 [Parameter(Mandatory)][string]$OutputDirectory,
 [Parameter(Mandatory)][string]$TemporaryDirectory,
 [ValidateSet('ely-doc','ely-table','ely-show')][string]$Application='ely-table',
 [ValidateRange(1,10)][int]$Cycles=4,
 [switch]$StopOnError
)
$ErrorActionPreference='Stop'
if(-not$IsWindows){throw 'Windows is required'}
foreach($name in @('BinaryDirectory','KernelDirectory','InputFile','OutputDirectory','TemporaryDirectory')){
 Set-Variable -Name $name -Value (Resolve-Path -LiteralPath (Get-Variable -Name $name -ValueOnly)).ProviderPath
}
$binary=Join-Path $BinaryDirectory "$Application.exe"
$worker=Join-Path $BinaryDirectory 'ely-lok-worker.exe'
$binaryHash=(Get-FileHash -LiteralPath $binary).Hash
$workerHash=(Get-FileHash -LiteralPath $worker).Hash
$inputHash=(Get-FileHash -LiteralPath $InputFile).Hash
Add-Type -Path "$PSScriptRoot/windows-native-ui.cs"
Add-Type -Path "$PSScriptRoot/windows-trial-job.cs"
. "$PSScriptRoot/windows-native-office-content.ps1"
. "$PSScriptRoot/windows-native-log.ps1"
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ElyCloseMemory {
 [StructLayout(LayoutKind.Sequential)] sealed class Memory {
  public uint length=(uint)Marshal.SizeOf<Memory>(),load;
  public ulong physical,availablePhysical,commit,availableCommit,virtualBytes,availableVirtual,extended;
 }
 [DllImport("kernel32.dll")] static extern bool GlobalMemoryStatusEx([In,Out]Memory m);
 public static ulong[] Available(){var m=new Memory();if(!GlobalMemoryStatusEx(m))throw new Exception("Memory snapshot failed");return new[]{m.availableCommit,m.availablePhysical};}
}
'@
$suite=Join-Path $OutputDirectory ('close-probe-'+[guid]::NewGuid().ToString('N').Substring(0,8))
New-Item -ItemType Directory -Path $suite|Out-Null
$records=@()
for($cycle=1;$cycle -le $Cycles;$cycle++){
 $case=Join-Path $suite "cycle-$cycle"
 $caseProfile=Join-Path $case 'profile'
 New-Item -ItemType Directory -Path $caseProfile|Out-Null
 $log=Join-Path $caseProfile "$Application/ely.log"
 $job=$null
 $r=[ordered]@{cycle=$cycle;completed=$false;passed=$false;forced_cleanup=$false;guard_reason=$null;peak_workers=0;minimum_commit_mib=[long]::MaxValue;minimum_physical_mib=[long]::MaxValue}
 try{
  $info=[Diagnostics.ProcessStartInfo]::new($binary)
  $info.UseShellExecute=$false
  $info.ArgumentList.Add($InputFile)
  foreach($pair in @{ELY_DATA_DIR=$caseProfile;ELY_LOK_DIR=$KernelDirectory;ELY_LOK_WORKER=$worker;ELY_SMOKE='hold';ELY_NO_DIALOG='1';ELY_LOCALE='en';TEMP=$TemporaryDirectory;TMP=$TemporaryDirectory}.GetEnumerator()){$info.Environment[$pair.Key]=$pair.Value}
  $job=[ElyTrialJob]::Start($info)
  $app=$job.Root
  $r.pid=$app.Id
  $deadline=[DateTime]::UtcNow.AddSeconds(45)
  $handle=[IntPtr]0
  $closing=$false
  while(-not$app.HasExited){
   $memory=[ElyCloseMemory]::Available()
   $r.minimum_commit_mib=[math]::Min($r.minimum_commit_mib,[long]($memory[0]/1MB))
   $r.minimum_physical_mib=[math]::Min($r.minimum_physical_mib,[long]($memory[1]/1MB))
   $workers=0
   foreach($id in $job.ProcessIds){try{if([Diagnostics.Process]::GetProcessById($id).ProcessName -eq 'ely-lok-worker'){$workers++}}catch [ArgumentException]{}}
   $r.peak_workers=[math]::Max($r.peak_workers,$workers)
   if($workers -gt 1 -or $memory[0] -lt 2GB -or $memory[1] -lt 1GB){$r.guard_reason='worker or memory limit';throw $r.guard_reason}
   if(-not$closing){
    $app.Refresh();$handle=$app.MainWindowHandle
    $text=Read-ElyNativeLog $log
    if($handle -ne [IntPtr]0 -and (Test-ElyOpenedInput $text $InputFile)){
     $r.window_dpi=[ElyNativeUI]::GetDpiForWindow($handle)
     [ElyNativeUI]::Close($handle)
     $closing=$true
     $deadline=[DateTime]::UtcNow.AddSeconds(15)
    }
   }
   if([DateTime]::UtcNow -gt $deadline){throw 'Owned window open/close timed out'}
   Start-Sleep -Milliseconds 100
  }
  $r.exit_code=$app.ExitCode
  if($r.exit_code -ne 0){throw 'Owned app exited with an error'}
  $deadline=[DateTime]::UtcNow.AddSeconds(2)
  while($job.Active -gt 0 -and [DateTime]::UtcNow -lt $deadline){Start-Sleep -Milliseconds 100}
  if($job.Active -gt 0){throw 'Owned app left descendants'}
  if(-not$closing){throw 'App exited before the requested close'}
  $r.job_empty=$true
  if((Get-FileHash -LiteralPath $InputFile).Hash -ne $inputHash){throw 'Probe rewrote input'}
  $r.original_unchanged=$true
  $r.completed=$true
 }catch{$r.error=$_.Exception.Message}
 finally{
  if($job){
   if($job.Active -gt 0){$job.Stop();$r.forced_cleanup=$true}
   if([Threading.Tasks.Task]::WaitAll([Threading.Tasks.Task[]]@($job.Stdout,$job.Stderr),5000)){
    $job.Stdout.GetAwaiter().GetResult()|Set-Content -LiteralPath "$case/stdout.log" -Encoding utf8
    $stderr=$job.Stderr.GetAwaiter().GetResult()
    $stderr|Set-Content -LiteralPath "$case/stderr.log" -Encoding utf8
    $r.error_lines=@([regex]::Matches($stderr,'(?m)^.*\bERROR\b[^\r\n]*')|ForEach-Object Value)
    $r.invalid_handle_count=[regex]::Matches($stderr,'(?i)0x80070578').Count
    $r.passed=$r.completed -and -not$r.forced_cleanup -and $r.error_lines.Count -eq 0 -and $r.invalid_handle_count -eq 0
   }else{$r.error='Owned output did not close';$r.completed=$false}
   $job.Dispose()
  }
  $records+=@{directory=$case;result=$r}
  @{app=$Application;binary=$binary;binary_sha256=$binaryHash;worker_sha256=$workerHash;input=$InputFile;input_sha256=$inputHash;checker_sha256=(Get-FileHash -LiteralPath $PSCommandPath).Hash;requested_cycles=$Cycles;cycles=$records;passed=($records.Count -eq $Cycles -and @($records|Where-Object {-not$_.result.passed}).Count -eq 0)}|ConvertTo-Json -Depth 7|Set-Content -LiteralPath "$suite/results.json" -Encoding utf8
 }
 if($StopOnError -and -not$r.passed){break}
}
Write-Output "$suite/results.json"
if(@($records|Where-Object {-not$_.result.passed}).Count -or $records.Count -ne $Cycles){exit 1}
