#Requires -Version 7.2
param(
 [Parameter(Mandatory)][string]$BinaryDirectory,
 [Parameter(Mandatory)][string]$KernelDirectory,
 [Parameter(Mandatory)][string]$InputFile,
 [Parameter(Mandatory)][string]$OutputDirectory,
 [Parameter(Mandatory)][string]$TemporaryDirectory,
 [Parameter(Mandatory)][string]$ReportFile,
 [ValidateRange(1,10)][int]$Cycles=2
)
$ErrorActionPreference='Stop'
if(-not$IsWindows){throw 'Windows is required'}
foreach($name in @('BinaryDirectory','KernelDirectory','InputFile','TemporaryDirectory')){
 Set-Variable -Name $name -Value (Resolve-Path -LiteralPath (Get-Variable -Name $name -ValueOnly)).ProviderPath
}
$OutputDirectory=[IO.Path]::GetFullPath($OutputDirectory)
$ReportFile=[IO.Path]::GetFullPath($ReportFile)
if(Test-Path -LiteralPath $ReportFile){throw 'Owned diagnostic report already exists'}
$output=$OutputDirectory
if(Test-Path -LiteralPath $output){throw 'Minimized probe already exists'}
New-Item -ItemType Directory -Path $output|Out-Null
Add-Type -Path "$PSScriptRoot/windows-trial-job.cs"
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ElyOwnedWindowState {
 [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h,int command);
 [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
}
'@
$probe=(Resolve-Path -LiteralPath "$PSScriptRoot/windows-close-probe.ps1").ProviderPath
$info=[Diagnostics.ProcessStartInfo]::new((Get-Command pwsh).Source)
$info.UseShellExecute=$false;$info.CreateNoWindow=$true
foreach($arg in @('-NoProfile','-File',$probe,'-BinaryDirectory',$BinaryDirectory,'-KernelDirectory',$KernelDirectory,'-InputFile',$InputFile,'-OutputDirectory',$output,'-TemporaryDirectory',$TemporaryDirectory,'-Application','ely-doc','-Cycles',"$Cycles",'-StopOnError')){$info.ArgumentList.Add($arg)}
$r=[ordered]@{completed=$false;intentional_minimize=$false;restored=$false;forced_cleanup=$false;target_pid=$null;harness_sha256=(Get-FileHash -LiteralPath $PSCommandPath).Hash;probe_sha256=(Get-FileHash -LiteralPath $probe).Hash;binary_sha256=(Get-FileHash -LiteralPath "$BinaryDirectory/ely-doc.exe").Hash}
$job=$null
try{
 $job=[ElyTrialJob]::Start($info)
 $root=$job.Root
 $deadline=[DateTime]::UtcNow.AddSeconds(30)
 $handle=[IntPtr]0
 while($handle -eq [IntPtr]0){
  foreach($id in $job.ProcessIds){try{
   $p=[Diagnostics.Process]::GetProcessById($id)
   if($p.ProcessName -eq 'ely-doc'){$p.Refresh();if($p.MainWindowHandle -ne [IntPtr]0){$handle=$p.MainWindowHandle;$r.target_pid=$p.Id;break}}
  }catch [ArgumentException]{}}
  if($root.HasExited -or [DateTime]::UtcNow -gt $deadline){throw 'Owned Doc window did not appear'}
  Start-Sleep -Milliseconds 20
 }
 $r.owned_pids_at_injection=@($job.ProcessIds)
 if($r.target_pid -notin $r.owned_pids_at_injection){throw 'Target window does not belong to the diagnostic job'}
 [void][ElyOwnedWindowState]::ShowWindowAsync($handle,6)
 $deadline=[DateTime]::UtcNow.AddSeconds(8)
 while(-not[ElyOwnedWindowState]::IsIconic($handle)){if([DateTime]::UtcNow -gt $deadline){throw 'Owned window did not minimize'};Start-Sleep -Milliseconds 20}
 $r.intentional_minimize=$true
 $deadline=[DateTime]::UtcNow.AddSeconds(25)
 $text=''
 $openedPattern='opened '+[regex]::Escape([IO.Path]::GetFileName($InputFile))+' as tab \d+, risk '+''
 while($text -notmatch $openedPattern){
  $dir=Get-ChildItem -LiteralPath $output -Directory|Select-Object -First 1
  if($dir){$log=Join-Path $dir.FullName 'cycle-1/profile/ely-doc/ely.log';if(Test-Path -LiteralPath $log){$text=Get-Content -LiteralPath $log -Raw}}
  if($root.HasExited -or [DateTime]::UtcNow -gt $deadline){throw 'Owned minimized Doc did not load'}
  Start-Sleep -Milliseconds 50
 }
 Start-Sleep -Milliseconds 700
 $text=Get-Content -LiteralPath $log -Raw
 $r.opened_while_minimized=[ElyOwnedWindowState]::IsIconic($handle)
 $r.first_tile_before_restore=[regex]::Matches($text,'first tile drawn').Count
 if(-not$r.opened_while_minimized -or $r.first_tile_before_restore -ne 0){throw 'This run did not reproduce the minimized first-tile boundary'}
 [void][ElyOwnedWindowState]::ShowWindowAsync($handle,9)
 $r.restored=$true
 if(-not$root.WaitForExit(120000)){throw 'Owned restored probe did not finish'}
 if($root.ExitCode -ne 0){throw 'Original strict close probe failed after restore'}
 if($job.Active -gt 0){throw 'Diagnostic job left descendants'}
 if(-not[Threading.Tasks.Task]::WaitAll([Threading.Tasks.Task[]]@($job.Stdout,$job.Stderr),5000)){throw 'Owned diagnostic output did not drain'}
 $out=$job.Stdout.GetAwaiter().GetResult()
 $out|Set-Content -LiteralPath "$ReportFile.stdout.log" -Encoding utf8
 $job.Stderr.GetAwaiter().GetResult()|Set-Content -LiteralPath "$ReportFile.stderr.log" -Encoding utf8
 $result=($out -split '\r?\n'|Where-Object {$_ -like '*results.json'}|Select-Object -Last 1).Trim()
 $p=Get-Content -LiteralPath $result -Raw|ConvertFrom-Json
 if(-not$p.passed -or $p.cycles.Count -ne $Cycles -or @($p.cycles|Where-Object {$_.result.forced_cleanup -or -not$_.result.passed}).Count){throw 'Actual close cycles were not accepted'}
 $r.probe_result=$result;$r.probe_sha256=$p.checker_sha256
 $r.completed=$true
}catch{$r.error=$_.Exception.Message}
finally{
 if($job){if($job.Active -gt 0){$job.Stop();$r.forced_cleanup=$true};$job.Dispose()}
 $r|ConvertTo-Json -Depth 5|Set-Content -LiteralPath $ReportFile -Encoding utf8
}
if(-not$r.completed -or $r.forced_cleanup){throw 'Minimized/restore diagnostic did not pass'}
"Owned minimize/load/no-tile/restore followed by $Cycles original strict close cycles passed."
