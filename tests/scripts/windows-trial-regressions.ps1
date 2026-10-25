#Requires -Version 7.2
param(
 [Parameter(Mandatory)][string]$Binary,
 [Parameter(Mandatory)][string]$KernelDirectory,
 [Parameter(Mandatory)][string]$Worker,
 [string]$OutputDirectory=([IO.Path]::GetTempPath()),
 [string]$TemporaryDirectory=([IO.Path]::GetTempPath()),
 [string]$Filter='tests::',
 [ValidatePattern('^[A-Za-z0-9_-]+$')][string]$Label='office',
 [switch]$Exact,
 [ValidateRange(1,32)][int]$MaxWorkers=4,
 [ValidateRange(1,120)][int]$MaxMinutes=20
)
$ErrorActionPreference='Stop'
if(-not $IsWindows){throw 'This runner requires Windows'}
$Binary=(Resolve-Path -LiteralPath $Binary).ProviderPath
$Worker=(Resolve-Path -LiteralPath $Worker).ProviderPath
$KernelDirectory=(Resolve-Path -LiteralPath $KernelDirectory).ProviderPath
$OutputDirectory=(Resolve-Path -LiteralPath $OutputDirectory).ProviderPath
$TemporaryDirectory=(Resolve-Path -LiteralPath $TemporaryDirectory).ProviderPath
if(-not (Test-Path -LiteralPath $Binary -PathType Leaf)){throw 'Missing test executable'}
if(-not (Test-Path -LiteralPath $Worker -PathType Leaf)){throw 'Missing worker executable'}
if(-not (Test-Path -LiteralPath $KernelDirectory -PathType Container)){throw 'Missing kernel directory'}
if(-not (Test-Path -LiteralPath $OutputDirectory -PathType Container)){throw 'Missing output directory'}
if(-not (Test-Path -LiteralPath $TemporaryDirectory -PathType Container)){throw 'Missing temporary directory'}
Add-Type -Path (Join-Path $PSScriptRoot 'windows-trial-job.cs')
Add-Type @'
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
public static class ElyTrialStats {
 [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] struct Entry {
  public uint size,usage,pid;public UIntPtr heap;public uint module,threads,parent;
  public int priority;public uint flags;
  [MarshalAs(UnmanagedType.ByValTStr,SizeConst=260)]public string name;
 }
 [StructLayout(LayoutKind.Sequential)] sealed class Memory {
  public uint length=(uint)Marshal.SizeOf<Memory>(),load;
  public ulong physical,availablePhysical,commit,availableCommit,virtualBytes,availableVirtual,extended;
 }
 [DllImport("kernel32.dll",SetLastError=true)] static extern IntPtr CreateToolhelp32Snapshot(uint flags,uint pid);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode)] static extern bool Process32FirstW(IntPtr h,ref Entry e);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode)] static extern bool Process32NextW(IntPtr h,ref Entry e);
 [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);
 [DllImport("kernel32.dll")] static extern bool GlobalMemoryStatusEx([In,Out]Memory m);
 public static long[] Sample(uint root,string workerName) {
  var entries=new List<Entry>();var snapshot=CreateToolhelp32Snapshot(2,0);
  if(snapshot==(IntPtr)(-1))throw new Exception("Process snapshot failed");
  try{var entry=new Entry{size=(uint)Marshal.SizeOf<Entry>()};
   if(Process32FirstW(snapshot,ref entry)){do{entries.Add(entry);}while(Process32NextW(snapshot,ref entry));}
  }finally{CloseHandle(snapshot);}
  var owned=new HashSet<uint>{root};bool added;
  do{added=false;foreach(var e in entries)if(owned.Contains(e.parent)&&owned.Add(e.pid))added=true;}while(added);
  long bytes=0,workers=0,rootBytes=0;
  foreach(var e in entries)if(owned.Contains(e.pid)){
   try{using(var p=Process.GetProcessById((int)e.pid)){
    bytes+=p.PrivateMemorySize64;if(e.pid==root)rootBytes=p.PrivateMemorySize64;
    if(e.pid!=root&&e.name.Equals(workerName,StringComparison.OrdinalIgnoreCase))workers++;
   }}catch(ArgumentException){}catch(InvalidOperationException){}
  }
  var memory=new Memory();if(!GlobalMemoryStatusEx(memory))throw new Exception("Memory snapshot failed");
  return new[]{bytes,(long)owned.Count-1,workers,(long)memory.availablePhysical,(long)memory.availableCommit,rootBytes};
 }
}
'@
$suffix=[guid]::NewGuid().ToString('N').Substring(0,8)
$suite=Join-Path $OutputDirectory "ely-trial-$Label-$(Get-Date -Format 'yyyyMMdd-HHmmss')-$suffix"
$temporary=Join-Path $TemporaryDirectory "ely-test-$suffix"
New-Item -ItemType Directory -Path $suite | Out-Null
New-Item -ItemType Directory -Path $temporary | Out-Null
$info=[System.Diagnostics.ProcessStartInfo]::new($Binary)
$info.UseShellExecute=$false
$info.CreateNoWindow=$true
$info.RedirectStandardOutput=$true
$info.RedirectStandardError=$true
if($Filter){$info.ArgumentList.Add($Filter)}
if($Exact){$info.ArgumentList.Add('--exact')}
$info.ArgumentList.Add('--test-threads=1')
$info.ArgumentList.Add('--nocapture')
$info.Environment['ELY_DATA_DIR']="$temporary/profile"
$info.Environment['ELY_LOK_DIR']=$KernelDirectory
$info.Environment['ELY_LOK_WORKER']=$Worker
$info.Environment['ELY_NO_DIALOG']='1'
$info.Environment['TEMP']=$temporary
$info.Environment['TMP']=$temporary
$hash=(Get-FileHash -LiteralPath $Binary -Algorithm SHA256).Hash
$started=[DateTime]::UtcNow
$job=[ElyTrialJob]::Start($info)
$child=$job.Root
$stdout=$job.Stdout
$stderr=$job.Stderr
$workerName=[IO.Path]::GetFileName($Worker)
$result=[ordered]@{binary=$Binary;sha256=$hash;label=$Label;temporary_directory=$temporary;filter=$Filter;exact=[bool]$Exact;pid=$child.Id;started_utc=$started.ToString('o');forced_cleanup=$false;guard_reason=$null;peak_private_mib=0;peak_root_private_mib=0;peak_workers=0;minimum_available_commit_mib=[long]::MaxValue;minimum_available_physical_mib=[long]::MaxValue}
$samples=[System.Collections.Generic.List[object]]::new()
try {
 while(-not $child.HasExited){
  $values=[ElyTrialStats]::Sample([uint32]$child.Id,$workerName)
  $private=[math]::Round($values[0]/1MB)
  $rootPrivate=[math]::Round($values[5]/1MB)
  $physical=[math]::Round($values[3]/1MB)
  $commit=[math]::Round($values[4]/1MB)
  $result.peak_private_mib=[math]::Max($result.peak_private_mib,$private)
  $result.peak_root_private_mib=[math]::Max($result.peak_root_private_mib,$rootPrivate)
  $result.peak_workers=[math]::Max($result.peak_workers,$values[2])
  $result.minimum_available_commit_mib=[math]::Min($result.minimum_available_commit_mib,$commit)
  $result.minimum_available_physical_mib=[math]::Min($result.minimum_available_physical_mib,$physical)
  $samples.Add([pscustomobject]@{seconds=[math]::Round(([DateTime]::UtcNow-$started).TotalSeconds,1);private_mib=$private;workers=$values[2];descendants=$values[1];available_commit_mib=$commit})
  if($values[2] -gt $MaxWorkers){$result.guard_reason='worker count exceeded the test budget';throw $result.guard_reason}
  if($commit -lt 2048){$result.guard_reason='system commit headroom below 2 GiB';throw $result.guard_reason}
  if($physical -lt 1024){$result.guard_reason='available physical memory below 1 GiB';throw $result.guard_reason}
  if(([DateTime]::UtcNow-$started).TotalMinutes -gt $MaxMinutes){$result.guard_reason='test time limit';throw $result.guard_reason}
  Start-Sleep -Milliseconds 500
 }
 $child.WaitForExit()
 $result.exit_code=$child.ExitCode
 $result.completed=$true
} catch {$result.error=$_.Exception.Message;throw}
finally {
 $out='';$err=''
 try {
  if($child.HasExited){
   $drainDeadline=[DateTime]::UtcNow.AddSeconds(2)
   while($job.Active -gt 0 -and [DateTime]::UtcNow -lt $drainDeadline){Start-Sleep -Milliseconds 20}
  }
  if($job.Active -gt 0){
   $result.surviving_processes=@($job.ProcessIds | ForEach-Object {
    $member=Get-Process -Id $_ -ErrorAction SilentlyContinue
    [pscustomobject]@{pid=$_;name=$member.ProcessName}
   })
   $job.Stop()
   $result.forced_cleanup=$true
   if(-not $result.guard_reason){$result.guard_reason='test left descendant processes behind'}
   $result.completed=$false
   $result.exit_code=1
  }
  if(-not $child.WaitForExit(5000)){throw 'Test process did not terminate'}
  $result.root_exit_code=$child.ExitCode
  if(-not [Threading.Tasks.Task]::WaitAll([Threading.Tasks.Task[]]@($stdout,$stderr),5000)){throw 'Test output did not close'}
  $out=$stdout.GetAwaiter().GetResult()
  $err=$stderr.GetAwaiter().GetResult()
 } catch {$result.cleanup_error=$_.Exception.Message;$result.exit_code=1}
 finally {
  $job.Dispose()
  $out | Set-Content -LiteralPath "$suite/stdout.log" -Encoding utf8
  $err | Set-Content -LiteralPath "$suite/stderr.log" -Encoding utf8
  $result.reported_skip=$out -match '\bSKIP\b' -or $err -match '\bSKIP\b'
  $result.elapsed_seconds=[math]::Round(([DateTime]::UtcNow-$started).TotalSeconds,1)
  $samples | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath "$suite/resources.json" -Encoding utf8
  $result | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath "$suite/results.json" -Encoding utf8
  Write-Output "$suite/results.json"
  if($result.completed){Write-Output ($out -split '\r?\n' | Where-Object {$_ -match '^test result:|^    tests::'})}
 }
}
exit $result.exit_code
