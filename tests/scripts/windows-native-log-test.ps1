#Requires -Version 7.2
param([string]$OutputDirectory=([IO.Path]::GetTempPath()),[int]$ExpectedFailures=0)
$ErrorActionPreference='Stop'
$root=Join-Path (Resolve-Path -LiteralPath $OutputDirectory).ProviderPath ('native-log-controls-'+[guid]::NewGuid().ToString('N').Substring(0,8))
New-Item -ItemType Directory -Path $root|Out-Null
$script:log=Join-Path $root 'owned.log'
function Parse-Subject([string]$Name) {
 $tokens=$null;$errors=$null
 $ast=[Management.Automation.Language.Parser]::ParseInput([IO.File]::ReadAllText((Join-Path $PSScriptRoot $Name)),[ref]$tokens,[ref]$errors)
 if($errors.Count){throw "Subject does not parse: $Name"}
 return $ast
}
$recovery=Parse-Subject 'windows-native-recovery-trial.ps1'
$office=Parse-Subject 'windows-native-office-trial.ps1'
$close=Parse-Subject 'windows-close-probe.ps1'
function Find-Function($Ast,[string]$Name) {
 $nodes=@($Ast.FindAll({param($n) $n -is [Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq $Name},$true))
 if($nodes.Count -ne 1){throw "Need one function: $Name"}
 return $nodes[0]
}
Set-Item Function:/Read-TestRecovery -Value ((Find-Function $recovery 'Log-Text').Body.GetScriptBlock())
Set-Item Function:/Read-TestOffice -Value ((Find-Function $office 'Log-Text').Body.GetScriptBlock())
$start=Find-Function $recovery 'Start-App'
$script:baseline=@($start.Body.FindAll({param($n) $n -is [Management.Automation.Language.AssignmentStatementAst] -and $n.Left.Extent.Text -in @('$script:prefix','$script:offset')},$true)|Sort-Object {$_.Extent.StartOffset}|ForEach-Object {[ScriptBlock]::Create($_.Extent.Text)})
if($baseline.Count -notin @(1,2)){throw 'Unexpected phase-baseline structure'}
$read=@($close.FindAll({param($n) $n -is [Management.Automation.Language.AssignmentStatementAst] -and $n.Left.Extent.Text -eq '$text'},$true))
if($read.Count -ne 1){throw 'Need the actual close log read'}
$script:closeRead=[ScriptBlock]::Create($read[0].Extent.Text)
$helper=Join-Path $PSScriptRoot 'windows-native-log.ps1'
if(Test-Path -LiteralPath $helper){. $helper}
function Get-Content {
 param([string]$LiteralPath,[switch]$Raw,[string]$ErrorAction)
 if($LiteralPath -ne $script:log){throw 'Unexpected control read'}
 if($script:readError){throw 'synthetic read failure'}
 if($script:live){return Microsoft.PowerShell.Management\Get-Content -LiteralPath $LiteralPath -Raw -ErrorAction Stop}
 return $script:chunks
}
function Set-Baseline([string[]]$Parts) {
 $script:chunks=$Parts
 foreach($statement in $script:baseline){. $statement}
}
function Assert-Text($Actual,[string]$Expected) {
 if($Actual -isnot [string] -or -not[string]::Equals($Actual,$Expected,[StringComparison]::Ordinal)){throw 'Expected one exact string'}
}
function Reject([scriptblock]$Read,[string]$Reason) {
 $errorText=''
 try{&$Read|Out-Null}catch{$errorText=$_.Exception.Message}
 if(-not$errorText.Contains($Reason)){throw "Expected rejection: $Reason; observed: $errorText"}
}
$fixturePrefix="start 中文 😀`n"
$marker="opened input.xlsx as tab 0, risk None`nfirst tile drawn"
$cases=@(
 @{name='single-string-tail';run={Set-Baseline @($fixturePrefix);$script:chunks=@($fixturePrefix+'next');Assert-Text (Read-TestRecovery) 'next'}},
 @{name='tail-spans-provider-parts';run={Set-Baseline @($fixturePrefix);$script:chunks=@($fixturePrefix,'ne','xt');Assert-Text (Read-TestRecovery) 'next'}},
 @{name='opened-marker-spans-parts';run={Set-Baseline @('');$script:chunks=@('opened in',"put.xlsx as tab 0, risk None`nfirst tile ",'drawn');Assert-Text (Read-TestRecovery) $marker}},
 @{name='unicode-parts-keep-code-units';run={Set-Baseline @('');$script:chunks=@(('中'+[char]0xd83d),([string][char]0xde00+'文'));Assert-Text (Read-TestRecovery) '中😀文'}},
 @{name='phase-baseline-uses-character-length';run={Set-Baseline @('start ' ,"中文 😀`n");if($script:offset -ne $fixturePrefix.Length){throw 'Baseline counted parts instead of characters'}}},
 @{name='real-truncation-still-rejected';run={Set-Baseline @($fixturePrefix);$script:live=$true;[IO.File]::WriteAllText($log,'short');Reject {Read-TestRecovery} 'truncated'}},
 @{name='same-length-prefix-replacement-rejected';run={Set-Baseline @($fixturePrefix);$script:live=$true;[IO.File]::WriteAllText($log,('X'+$fixturePrefix.Substring(1)+'next'));Reject {Read-TestRecovery} 'prefix'}},
 @{name='missing-after-baseline-rejected';run={Set-Baseline @($fixturePrefix);Remove-Item -LiteralPath $log;Reject {Read-TestRecovery} 'truncated'}},
 @{name='read-errors-propagate';run={Set-Baseline @('');$script:readError=$true;Reject {Read-TestRecovery} 'synthetic read failure'}},
 @{name='missing-initial-log-waits';run={Set-Baseline @('');Remove-Item -LiteralPath $log;Assert-Text (Read-TestRecovery) ''}},
 @{name='unchanged-baseline-has-empty-tail';run={Set-Baseline @($fixturePrefix);$script:chunks=@($fixturePrefix);Assert-Text (Read-TestRecovery) ''}},
 @{name='empty-existing-log-is-one-string';run={Set-Baseline @('');$script:live=$true;[IO.File]::WriteAllText($log,'');Assert-Text (Read-TestRecovery) ''}},
 @{name='office-reader-keeps-marker-parts';run={$script:chunks=@('opened in',"put.xlsx as tab 0, risk None`nfirst tile ",'drawn');Assert-Text (Read-TestOffice) $marker}},
 @{name='close-reader-keeps-marker-parts';run={$script:chunks=@('opened in',"put.xlsx as tab 0, risk None`nfirst tile ",'drawn');. $script:closeRead;Assert-Text $text $marker}}
)
$results=@()
foreach($case in $cases){
 [IO.File]::WriteAllText($log,'fixture')
 $script:live=$false;$script:readError=$false;$script:chunks=@();$script:offset=0;$script:prefix=''
 $script:r=@{log_reads=@{reads=0;chunked_reads=0;max_chunks=0}}
 $message=''
 try{&$case.run}catch{$message=$_.Exception.Message}
 $results+=@{name=$case.name;passed=($message -eq '');error=$message}
}
$failed=@($results|Where-Object {-not$_.passed}).Count
if($cases.Count -ne 14){throw 'Control selection changed'}
@{cases=$results;passed=14-$failed;failed=$failed;expected_failures=$ExpectedFailures;test_sha256=(Get-FileHash -LiteralPath $PSCommandPath).Hash}|ConvertTo-Json -Depth 5|Set-Content -LiteralPath "$root/results.json" -Encoding utf8
Write-Output "$root/results.json"
Write-Output "Controls: $(14-$failed) passed, $failed failed; these are not application tests"
if($failed -ne $ExpectedFailures){exit 1}
