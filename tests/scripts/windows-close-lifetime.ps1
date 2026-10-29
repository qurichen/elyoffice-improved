#Requires -Version 7.2
param(
 [Parameter(Mandatory)][string]$SuiteResult,
 [Parameter(Mandatory)][string]$BinaryDirectory,
 [Parameter(Mandatory)][string]$OutputFile
)
$ErrorActionPreference='Stop'
$SuiteResult=(Resolve-Path -LiteralPath $SuiteResult).ProviderPath
$BinaryDirectory=(Resolve-Path -LiteralPath $BinaryDirectory).ProviderPath
if(Test-Path -LiteralPath $OutputFile){throw 'Lifetime report already exists'}
$root=Split-Path -Parent $SuiteResult
$records=@(Get-Content -LiteralPath $SuiteResult -Raw|ConvertFrom-Json)
if(-not$records.Count){throw 'Native suite is empty'}
$checks=@()
foreach($record in $records){
 $r=$record.result
 if($r.app -notin @('ely-table','ely-show')){throw 'Unexpected native application'}
 $case=[IO.Path]::GetFullPath($record.suite)
 if(-not$case.StartsWith($root+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Native case escapes suite'}
 foreach($key in @('completed','default_extension_and_contents','cancel_then_ordinary_save_keeps_edits','declined_overwrite_preserves_target','explicit_format_contents','original_unchanged','reopen_unchanged','first_job_empty','reopen_job_empty')){
  if($r.$key -ne $true){throw "Native assertion failed: $key"}
 }
 if($r.forced_cleanup -ne $false -or $r.first_exit_code -ne 0 -or $r.reopen_exit_code -ne 0){throw 'Native exit or cleanup failed'}
 $binary=Join-Path $BinaryDirectory "$($r.app).exe"
 $worker=Join-Path $BinaryDirectory 'ely-lok-worker.exe'
 if((Get-FileHash -LiteralPath $binary).Hash -ne $r.binary_sha256 -or (Get-FileHash -LiteralPath $worker).Hash -ne $r.worker_sha256){throw 'Native binary identity differs'}
 $helpers=@{harness_sha256='windows-native-office-trial.ps1';reader_sha256='windows-native-office-content.ps1';ui_helper_sha256='windows-native-ui.cs';job_helper_sha256='windows-trial-job.cs'}
 foreach($key in $helpers.Keys){if((Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $helpers[$key])).Hash -ne $r.$key){throw "Native helper differs: $key"}}
 foreach($phase in @('first','reopen')){
  $path=Join-Path $case "$phase-stderr.log"
  $log=Get-Content -LiteralPath $path -Raw
  if(-not$log){throw 'Native stderr is empty'}
  $errors=@([regex]::Matches($log,'(?m)^.*\bERROR\b[^\r\n]*')|ForEach-Object Value)
  $invalid=[regex]::Matches($log,'(?i)0x80070578').Count
  $checks+=@{app=$r.app;phase=$phase;stderr=$path;stderr_sha256=(Get-FileHash -LiteralPath $path).Hash;error_lines=$errors;invalid_handle_count=$invalid;passed=($errors.Count -eq 0 -and $invalid -eq 0)}
 }
}
$passed=@($checks|Where-Object {-not $_.passed}).Count -eq 0
@{suite_result=$SuiteResult;suite_sha256=(Get-FileHash -LiteralPath $SuiteResult).Hash;checker_sha256=(Get-FileHash -LiteralPath $PSCommandPath).Hash;passed=$passed;checks=$checks}|ConvertTo-Json -Depth 6|Set-Content -LiteralPath $OutputFile -Encoding utf8
Write-Output $OutputFile
if(-not$passed){exit 1}
