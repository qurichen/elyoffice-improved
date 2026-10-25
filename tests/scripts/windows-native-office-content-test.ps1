#Requires -Version 7.2
param([string]$OutputDirectory=([IO.Path]::GetTempPath()))
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
. "$PSScriptRoot/windows-native-office-content.ps1"
$root=(Resolve-Path -LiteralPath $OutputDirectory).ProviderPath
$suite=Join-Path $root ('native-content-controls-'+[guid]::NewGuid().ToString('N').Substring(0,8))
New-Item -ItemType Directory -Path $suite | Out-Null
$marker='Marker 中文'
$second='Later draft'
$sst='<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>Marker 中文</t></si><si><t>Later draft</t></si><si><t>wrong</t></si></sst>'
$xlsx='<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1"><f>1+2</f><v>3</v></c><c r="C1"><f>3</f><v>3</v></c><c r="D1" t="s"><v>0</v></c></row><row r="2"><c r="A2" t="s"><v>1</v></c></row></sheetData></worksheet>'
$ods='<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"><office:body><office:spreadsheet><table:table><table:table-row><table:table-cell><text:p>Marker 中文</text:p></table:table-cell><table:table-cell table:formula="of:=1+2" office:value="3"/><table:table-cell office:value="3"/><table:table-cell><text:p>Marker 中文</text:p></table:table-cell></table:table-row><table:table-row><table:table-cell><text:p>Later draft</text:p></table:table-cell><table:table-cell><text:p>Later draft</text:p></table:table-cell></table:table-row></table:table></office:spreadsheet></office:body></office:document-content>'
$cases=@(
 @{name='xlsx-valid';xml=$xlsx;odf=$false;error=''},
 @{name='xlsx-unreferenced-marker';xml=$xlsx.Replace('r="A1" t="s"><v>0','r="A1" t="s"><v>2');odf=$false;error='Cell A1'},
 @{name='xlsx-unreferenced-later';xml=$xlsx.Replace('r="A2" t="s"><v>1','r="A2" t="s"><v>2');odf=$false;error='Cell A2'},
 @{name='xlsx-wrong-formula';xml=$xlsx.Replace('<f>1+2</f>','<f>11+2</f>');odf=$false;error='Cell B1'},
 @{name='xlsx-unrelated-value';xml=$xlsx.Replace('<f>1+2</f><v>3','<f>1+2</f><v>99');odf=$false;error='Cell B1'},
 @{name='xlsx-inline-string';xml=$xlsx.Replace('<c r="A1" t="s"><v>0</v></c>','<c r="A1" t="inlineStr"><is><t>Marker 中文</t></is></c>');odf=$false;error=''},
 @{name='ods-valid';xml=$ods;odf=$true;error=''},
 @{name='ods-unrelated-marker';xml=$ods.Replace('<text:p>Marker 中文</text:p></table:table-cell><table:table-cell table:formula','<text:p>wrong</text:p></table:table-cell><table:table-cell table:formula');odf=$true;error='Cell A1'},
 @{name='ods-unrelated-later';xml=$ods.Replace('<table:table-row><table:table-cell><text:p>Later draft','<table:table-row><table:table-cell><text:p>wrong');odf=$true;error='Cell A2'},
 @{name='ods-wrong-formula';xml=$ods.Replace('of:=1+2','of:=11+2');odf=$true;error='Cell B1'},
 @{name='ods-unrelated-value';xml=$ods.Replace('formula="of:=1+2" office:value="3"','formula="of:=1+2" office:value="99"');odf=$true;error='Cell B1'}
)
$results=@()
foreach($case in $cases){
 $path=Join-Path $suite ($case.name+'.zip')
 $zip=[IO.Compression.ZipFile]::Open($path,[IO.Compression.ZipArchiveMode]::Create)
 try{
  $parts=if($case.odf){@{'content.xml'=$case.xml}}else{@{'xl/worksheets/sheet1.xml'=$case.xml;'xl/sharedStrings.xml'=$sst}}
  foreach($part in $parts.GetEnumerator()){
   $writer=[IO.StreamWriter]::new($zip.CreateEntry($part.Key).Open())
   try{$writer.Write($part.Value)}finally{$writer.Dispose()}
  }
 }finally{$zip.Dispose()}
 $observed=''
 try{Assert-ElyOfficeContent $path 'table' $marker $second $true $case.odf}catch{$observed=$_.Exception.Message}
 if(($case.error -eq '' -and $observed) -or ($case.error -ne '' -and -not $observed.StartsWith($case.error))){throw "Unexpected reader result $($case.name): $observed"}
 $results+=@{case=$case.name;passed=$true;rejected=($case.error -ne '')}
}
$logs=@(
 @{name='wrong-opened-file';text="opened wrong.ods as tab 0, risk None`nfirst tile drawn";expected=$false},
 @{name='tile-before-requested-file';text="first tile drawn`nopened explicit.ods as tab 0, risk None";expected=$false},
 @{name='requested-file-then-tile';text="opened explicit.ods as tab 0, risk None`nfirst tile drawn";expected=$true}
)
foreach($case in $logs){
 if((Test-ElyOpenedInput $case.text 'explicit.ods') -ne $case.expected){throw "Incorrect opened-file check: $($case.name)"}
 $results+=@{case=$case.name;passed=$true;rejected=(-not $case.expected)}
}
$results|ConvertTo-Json -Depth 3|Set-Content -LiteralPath "$suite/results.json" -Encoding utf8
Write-Output "$suite/results.json"
Write-Output "$($results.Count) reader/log controls passed; these are not application tests"
