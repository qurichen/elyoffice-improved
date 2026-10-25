#Requires -Version 7.2
param(
 [Parameter(Mandatory)][string]$BinaryDirectory,
 [Parameter(Mandatory)][string]$KernelDirectory,
 [Parameter(Mandatory)][string]$OutputDirectory,
 [Parameter(Mandatory)][string]$TemporaryDirectory,
 [ValidateSet('table','show')][string[]]$Applications=@('table','show')
)
$ErrorActionPreference='Stop'
if(-not $IsWindows){throw 'Windows is required'}
foreach($name in @('BinaryDirectory','KernelDirectory','OutputDirectory','TemporaryDirectory')){
 Set-Variable -Name $name -Value (Resolve-Path -LiteralPath (Get-Variable -Name $name -ValueOnly)).ProviderPath
 if(-not (Test-Path -LiteralPath (Get-Variable -Name $name -ValueOnly) -PathType Container)){throw "Not a directory: $name"}
}
$worker=Join-Path $BinaryDirectory 'ely-lok-worker.exe'
if(-not (Test-Path -LiteralPath $worker -PathType Leaf)){throw 'Worker is missing'}
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type -Path "$PSScriptRoot/windows-native-ui.cs"
Add-Type -Path "$PSScriptRoot/windows-trial-job.cs"
. "$PSScriptRoot/windows-native-office-content.ps1"
$repo=Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$suite=Join-Path $OutputDirectory ('native-offices-'+[guid]::NewGuid().ToString('N').Substring(0,8))
New-Item -ItemType Directory -Path $suite | Out-Null
$records=@()
$job=$null
$app=$null
$handle=[IntPtr]0
$marker='Round13 中文表格 😀'
$second='Cancel keeps this draft 中文'

function Log-Text {
 if(Test-Path -LiteralPath $script:log){return Get-Content -LiteralPath $script:log -Raw}
 return ''
}
function Count-Log([string]$Pattern) { return [regex]::Matches((Log-Text),$Pattern).Count }
function Wait-Until([scriptblock]$Check,[int]$Seconds=25) {
 $deadline=[DateTime]::UtcNow.AddSeconds($Seconds)
 while([DateTime]::UtcNow -lt $deadline){
  if(&$Check){return}
  if($app.HasExited){throw "App exited at $($result.stage)"}
  Start-Sleep -Milliseconds 100
 }
 throw "Timed out at $($result.stage)"
}
function Capture([string]$Name,[IntPtr]$Window=$handle) {
 $rect=[ElyNativeUI]::PhysicalBounds($Window)
 $bitmap=[Drawing.Bitmap]::new($rect.R-$rect.L,$rect.B-$rect.T)
 $graphics=[Drawing.Graphics]::FromImage($bitmap)
 try{
  $dc=$graphics.GetHdc()
  try{$ok=[ElyNativeUI]::PrintWindow($Window,$dc,2)}finally{$graphics.ReleaseHdc($dc)}
  if(-not $ok){throw 'Owned window capture failed'}
  $bitmap.Save((Join-Path $case "$Name.png"))
 }finally{$graphics.Dispose();$bitmap.Dispose()}
}
function Key([int]$Code) {
 [ElyNativeUI]::Key($handle,[uint32]$Code)
}
function Type-Text([string]$Text) {
 foreach($character in $Text.ToCharArray()){
  if(-not [ElyNativeUI]::PostMessageW($handle,0x102,[IntPtr][int]$character,[IntPtr]0)){throw 'Owned character post failed'}
 }
}
function Clear-Field {
 Key 35
 for($index=0;$index -lt 40;$index++){Key 8}
}
function Write-Cell([string]$Address,[string]$Text) {
 $before=Count-Log 'ran cell.goto'
 [ElyNativeUI]::Click($handle,80,193)
 [ElyNativeUI]::Frame($handle)
 Clear-Field
 Type-Text $Address
 Key 13
 Wait-Until {(Count-Log 'ran cell.goto') -gt $before}
 [ElyNativeUI]::Frame($handle)
 $before=Count-Log 'ran cell.enter'
 [ElyNativeUI]::Click($handle,400,193)
 [ElyNativeUI]::Frame($handle)
 Clear-Field
 Type-Text $Text
 Key 13
 Wait-Until {(Count-Log 'ran cell.enter') -gt $before}
 [ElyNativeUI]::Frame($handle)
}
function Add-Slide {
 $before=Count-Log 'ran slide.insert'
 [ElyNativeUI]::Click($handle,232,87)
 [ElyNativeUI]::Frame($handle)
 [ElyNativeUI]::Click($handle,263,146)
 Wait-Until {(Count-Log 'ran slide.insert') -gt $before}
 [ElyNativeUI]::Frame($handle)
}
function File-Menu([bool]$SaveAs=$true) {
 [ElyNativeUI]::Click($handle,163,87)
 [ElyNativeUI]::Frame($handle)
 $y=if($SaveAs){293}else{244}
 [ElyNativeUI]::Click($handle,205,$y)
}
function Save-Panel([string]$Path,[switch]$Cancel,[switch]$Keep) {
 $script:panel=[IntPtr]0
 Wait-Until {$script:panel=[ElyNativeUI]::Dialog([uint32]$app.Id,1001,'Edit');$script:panel -ne [IntPtr]0}
 [ElyNativeUI]::Text([ElyNativeUI]::Child($panel,1001,'Edit'),[IO.Path]::GetFullPath($Path))
 if($Cancel){[ElyNativeUI]::Close($panel)}else{[ElyNativeUI]::Command($panel,1)}
 if(-not $Keep){Wait-Until {-not [ElyNativeUI]::IsWindow($panel)}}
}
function Check-Content([string]$Path,[bool]$AfterCancel,[bool]$OpenDocument=$false) {
 Assert-ElyOfficeContent $Path $application $marker $second $AfterCancel $OpenDocument
}
function Start-App([string]$InputFile,[string]$Profile,[string]$Phase) {
 $script:phase=$Phase
 $script:log=Join-Path $Profile "$slug/ely.log"
 $info=[Diagnostics.ProcessStartInfo]::new($binary)
 $info.UseShellExecute=$false
 $info.CreateNoWindow=$true
 $info.ArgumentList.Add($InputFile)
 $info.Environment['ELY_DATA_DIR']=$Profile
 $info.Environment['ELY_LOK_DIR']=$KernelDirectory
 $info.Environment['ELY_LOK_WORKER']=$worker
 $info.Environment['ELY_SMOKE']='hold'
 $info.Environment['ELY_NO_DIALOG']='1'
 $info.Environment['ELY_LOCALE']='en'
 $info.Environment['TEMP']=$TemporaryDirectory
 $info.Environment['TMP']=$TemporaryDirectory
 $script:job=[ElyTrialJob]::Start($info)
 $script:app=$job.Root
 $script:handle=[IntPtr]0
 Wait-Until {$app.Refresh();$script:handle=$app.MainWindowHandle;$handle -ne [IntPtr]0 -and (Test-ElyOpenedInput (Log-Text) $InputFile)} 45
 $result["${Phase}_opened_input"]=[IO.Path]::GetFileName($InputFile)
 $result["${Phase}_pid"]=$app.Id
 $dpi=[ElyNativeUI]::GetDpiForWindow($handle)
 if($dpi -ne 168){throw 'This coordinate-based trial supports only the validated 168 DPI layout'}
 $result.window_dpi=$dpi
 $result.window_geometry=[ElyNativeUI]::Geometry($handle)
 [ElyNativeUI]::Frame($handle)
}
function Finish-App {
 [ElyNativeUI]::Close($handle)
 if(-not $app.WaitForExit(15000)){throw 'Owned app did not close normally'}
 if($app.ExitCode -ne 0){throw 'Owned app exited with an error'}
 $result["${phase}_exit_code"]=$app.ExitCode
 $deadline=[DateTime]::UtcNow.AddSeconds(2)
 while($job.Active -gt 0 -and [DateTime]::UtcNow -lt $deadline){Start-Sleep -Milliseconds 100}
 if($job.Active -gt 0){throw 'Owned app left descendant processes'}
 $result["${phase}_job_empty"]=$true
 if(-not [Threading.Tasks.Task]::WaitAll([Threading.Tasks.Task[]]@($job.Stdout,$job.Stderr),5000)){throw 'Owned output did not close'}
 $job.Stdout.GetAwaiter().GetResult() | Set-Content -LiteralPath "$case/$phase-stdout.log" -Encoding utf8
 $job.Stderr.GetAwaiter().GetResult() | Set-Content -LiteralPath "$case/$phase-stderr.log" -Encoding utf8
 $job.Dispose()
 $script:job=$null
 $script:app=$null
}
foreach($application in ($Applications|Select-Object -Unique)){
 $slug="ely-$application"
 $extension=if($application -eq 'table'){'xlsx'}else{'pptx'}
 $alternative=if($application -eq 'table'){'ods'}else{'odp'}
 $binary=Join-Path $BinaryDirectory "$slug.exe"
 if(-not (Test-Path -LiteralPath $binary -PathType Leaf)){throw "Missing application: $slug"}
 $case=Join-Path $suite $slug
 New-Item -ItemType Directory -Path $case | Out-Null
 $source=Join-Path $case "input.$extension"
 $seed=Join-Path $repo "packaging/windows/new/new.$extension"
 Copy-Item -LiteralPath $seed -Destination $source
 $original=(Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
 $result=[ordered]@{app=$slug;binary_sha256=(Get-FileHash -LiteralPath $binary).Hash;worker_sha256=(Get-FileHash -LiteralPath $worker).Hash;stage='start';completed=$false;forced_cleanup=$false;system_ime_tested=$false}
 $result.harness_sha256=(Get-FileHash -LiteralPath $PSCommandPath).Hash
 $result.reader_sha256=(Get-FileHash -LiteralPath "$PSScriptRoot/windows-native-office-content.ps1").Hash
 $result.ui_helper_sha256=(Get-FileHash -LiteralPath "$PSScriptRoot/windows-native-ui.cs").Hash
 $result.job_helper_sha256=(Get-FileHash -LiteralPath "$PSScriptRoot/windows-trial-job.cs").Hash
 try{
  Start-App $source "$case/profile" 'first'
  Capture 'opened'
  $result.stage='edit'
  if($application -eq 'table'){Write-Cell 'A1' $marker;Write-Cell 'B1' '=1+2'}else{Add-Slide}
  $result.stage='default-save-as'
  $before=Count-Log 'office: saved '
  File-Menu
  $target=Join-Path $case '中文 no-extension'
  Save-Panel $target
  Wait-Until {(Count-Log 'office: saved ') -gt $before}
  $saved="$target.$extension"
  if((Test-Path -LiteralPath $target) -or -not (Test-Path -LiteralPath $saved)){throw 'Default extension differs'}
  Check-Content $saved $false
  $result.default_extension_and_contents=$true
  $result.stage='cancel-preserves-draft'
  if($application -eq 'table'){Write-Cell 'A2' $second}else{Add-Slide}
  File-Menu
  $cancel=Join-Path $case 'cancelled'
  Save-Panel $cancel -Cancel
  Wait-Until {(Log-Text) -match 'save panel was cancelled'}
  if((Test-Path -LiteralPath $cancel) -or (Test-Path -LiteralPath "$cancel.$extension")){throw 'Cancel created a target'}
  $before=Count-Log 'office: saved '
  File-Menu $false
  Wait-Until {(Count-Log 'office: saved ') -gt $before}
  Check-Content $saved $true
  $result.cancel_then_ordinary_save_keeps_edits=$true
  $result.stage='decline-overwrite'
  $guard=Join-Path $case "guard.$extension"
  Copy-Item -LiteralPath $seed -Destination $guard
  $guardHash=(Get-FileHash -LiteralPath $guard).Hash
  File-Menu
  Save-Panel (Join-Path $case 'guard') -Keep
  $script:confirm=[IntPtr]0
  Wait-Until {$script:confirm=[ElyNativeUI]::Confirmation([uint32]$app.Id);$confirm -ne [IntPtr]0}
  Wait-Until {[ElyNativeUI]::IsWindowVisible($confirm)}
  Capture 'overwrite-confirmation' $confirm
  [void][ElyNativeUI]::SendMessageW([ElyNativeUI]::NoButton($confirm),0xF5,[IntPtr]0,[IntPtr]0)
  Wait-Until {-not [ElyNativeUI]::IsWindow($confirm)}
  [ElyNativeUI]::Close($panel)
  Wait-Until {-not [ElyNativeUI]::IsWindow($panel)}
  if((Get-FileHash -LiteralPath $guard).Hash -ne $guardHash){throw 'Declined overwrite changed the target'}
  $result.declined_overwrite_preserves_target=$true
  $result.stage='explicit-open-document'
  $before=Count-Log 'office: saved '
  File-Menu
  $other=Join-Path $case "explicit.$alternative"
  Save-Panel $other
  Wait-Until {(Count-Log 'office: saved ') -gt $before}
  if(Test-Path -LiteralPath "$other.$extension"){throw 'Explicit format gained an unwanted suffix'}
  Check-Content $other $true $true
  $result.explicit_format_contents=$true
  Capture 'saved'
  $result.stage='close'
  Finish-App
  if((Get-FileHash -LiteralPath $source).Hash -ne $original){throw 'Save As changed the original input'}
  $result.original_unchanged=$true
  $otherHash=(Get-FileHash -LiteralPath $other).Hash
  $result.stage='reopen'
  Start-App $other "$case/reopen-profile" 'reopen'
  Capture 'reopened'
  Finish-App
  Check-Content $other $true $true
  if((Get-FileHash -LiteralPath $other).Hash -ne $otherHash){throw 'Reopen rewrote the file'}
  $result.reopen_unchanged=$true
  $result.completed=$true
 }catch{
  $result.error=$_.Exception.Message
  if($app -and -not $app.HasExited){[ElyNativeUI]::Dump([uint32]$app.Id)|Set-Content -LiteralPath "$case/owned-windows.txt";try{Capture 'failure'}catch{}}
 }finally{
  if($job){
   if($job.Active -gt 0){$job.Stop();$result.forced_cleanup=$true}
   if([Threading.Tasks.Task]::WaitAll([Threading.Tasks.Task[]]@($job.Stdout,$job.Stderr),5000)){
    $job.Stdout.GetAwaiter().GetResult()|Set-Content -LiteralPath "$case/$phase-stdout.log" -Encoding utf8
    $job.Stderr.GetAwaiter().GetResult()|Set-Content -LiteralPath "$case/$phase-stderr.log" -Encoding utf8
   }
   $job.Dispose();$job=$null;$app=$null
  }
  $records+=@{suite=$case;result=$result}
  $records|ConvertTo-Json -Depth 5|Set-Content -LiteralPath "$suite/results.json" -Encoding utf8
 }
}
Write-Output "$suite/results.json"
if(@($records|Where-Object {-not $_.result.completed -or $_.result.forced_cleanup}).Count){exit 1}
