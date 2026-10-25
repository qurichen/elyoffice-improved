#Requires -Version 7.2
param(
 [string]$OutputDirectory=([IO.Path]::GetTempPath()),
 [string]$Compiler="$env:WINDIR/Microsoft.NET/Framework64/v4.0.30319/csc.exe"
)
$ErrorActionPreference='Stop'
if(-not $IsWindows){throw 'This test requires Windows'}
if(-not (Test-Path -LiteralPath $Compiler -PathType Leaf)){
 Write-Warning 'SKIP: the .NET Framework C# compiler is unavailable'
 exit 0
}
$output=(Resolve-Path -LiteralPath $OutputDirectory).ProviderPath
$suite=Join-Path $output ("ely-runner-controls-"+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $suite | Out-Null
$binary=Join-Path $suite 'control program.exe'
$worker=Join-Path $suite '改名 worker.exe'
$source=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'windows-trial-control.cs')).ProviderPath
& $Compiler /nologo /target:exe "/out:$binary" $source
if($LASTEXITCODE -ne 0){throw 'Could not compile the synthetic control'}
Copy-Item -LiteralPath $binary -Destination $worker
$runner=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot 'windows-trial-regressions.ps1')).ProviderPath
$quoted='参数 with "quotes" and trailing\'
$cases=@(
 @{mode=$quoted;label='quoted';exit=0;forced=$false},
 @{mode='orphan';label='orphan';exit=1;forced=$true},
 @{mode='limit';label='limit';exit=1;forced=$true}
)
$reports=@()
foreach($case in $cases){
 $info=[Diagnostics.ProcessStartInfo]::new((Join-Path $PSHOME 'pwsh.exe'))
 $info.UseShellExecute=$false
 $info.CreateNoWindow=$true
 $info.RedirectStandardOutput=$true
 $info.RedirectStandardError=$true
 foreach($argument in @('-NoProfile','-File',$runner,'-Binary',$binary,'-KernelDirectory',$suite,'-Worker',$worker,'-OutputDirectory',$suite,'-Filter',$case.mode,'-Label',$case.label,'-MaxWorkers','1')){
  $info.ArgumentList.Add($argument)
 }
 $child=[Diagnostics.Process]::Start($info)
 $stdout=$child.StandardOutput.ReadToEndAsync()
 $stderr=$child.StandardError.ReadToEndAsync()
 try {
  if(-not $child.WaitForExit(15000)){throw "Control timed out: $($case.label)"}
  if(-not [Threading.Tasks.Task]::WaitAll([Threading.Tasks.Task[]]@($stdout,$stderr),5000)){throw 'Control output did not close'}
  $out=$stdout.GetAwaiter().GetResult()
  $err=$stderr.GetAwaiter().GetResult()
  $out | Set-Content -LiteralPath "$suite/$($case.label)-stdout.log" -Encoding utf8
  $err | Set-Content -LiteralPath "$suite/$($case.label)-stderr.log" -Encoding utf8
  if($child.ExitCode -ne $case.exit){throw "Unexpected control exit: $($case.label)"}
  $paths=@($out -split '\r?\n' | Where-Object {$_ -match 'results[.]json$' -and (Test-Path -LiteralPath $_ -PathType Leaf)})
  if($paths.Count -ne 1){throw 'Missing control report'}
  $report=Get-Content -LiteralPath $paths[0] -Raw | ConvertFrom-Json
  if($report.forced_cleanup -ne $case.forced -or $report.cleanup_error){throw 'Unexpected cleanup result'}
  $folder=Split-Path -Parent $paths[0]
  $content=Get-Content -LiteralPath (Join-Path $folder 'stdout.log') -Raw
  if($case.label -eq 'quoted'){
   $errorContent=Get-Content -LiteralPath (Join-Path $folder 'stderr.log') -Raw
   if(-not $report.completed -or -not $content.Contains("synthetic-control=$quoted") -or -not $errorContent.Contains("synthetic-stderr=$quoted")){throw 'Argument or output preservation failed'}
  }
  if($case.label -eq 'orphan' -and ($report.root_exit_code -ne 0 -or $report.completed -or $report.guard_reason -ne 'test left descendant processes behind')){throw 'Orphan detection failed'}
  if($case.label -eq 'limit' -and ($report.peak_workers -lt 2 -or $report.guard_reason -ne 'worker count exceeded the test budget')){throw 'Renamed-worker limit failed'}
  foreach($match in [regex]::Matches($content,'owned-worker=(\d+)')){
   if(Get-Process -Id ([int]$match.Groups[1].Value) -ErrorAction SilentlyContinue){throw 'A synthetic child survived cleanup'}
  }
  $reports+=@{case=$case.label;passed=$true;report=$paths[0]}
 } finally {
  if(-not $child.HasExited){$child.Kill($true);[void]$child.WaitForExit(5000)}
  $child.Dispose()
 }
}
$reports | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath "$suite/results.json" -Encoding utf8
Write-Output "$suite/results.json"
Write-Output '3 runner controls passed; these are not application regressions'
