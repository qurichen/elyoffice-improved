#Requires -Version 7.2
param([string]$OutputDirectory=([IO.Path]::GetTempPath()))
$ErrorActionPreference='Stop'
. "$PSScriptRoot/windows-native-log.ps1"
Add-Type @'
using System;
using System.IO;
using System.Text;
using System.Threading.Tasks;
public static class ElyNativeLogAppend {
 public static Task Run(string path) {
  return Task.Run(() => {
   using(var f=new FileStream(path,FileMode.Append,FileAccess.Write,FileShare.ReadWrite)) {
    var line=Encoding.UTF8.GetBytes("append row 中文 😀\n");
    for(int i=0;i<30000;i++) { f.Write(line,0,line.Length); f.Flush(); }
   }
  });
 }
}
'@
$root=Join-Path (Resolve-Path -LiteralPath $OutputDirectory).ProviderPath ('native-log-append-'+[guid]::NewGuid().ToString('N').Substring(0,8))
New-Item -ItemType Directory -Path $root|Out-Null
$path=Join-Path $root 'append.log'
$fixturePrefix="baseline 中文 😀`n"*1000
$payload="append row 中文 😀`n"
[IO.File]::WriteAllText($path,$fixturePrefix,[Text.UTF8Encoding]::new($false))
$writer=[ElyNativeLogAppend]::Run($path)
$short=0;$arrays=0;$invalid=0;$examples=@()
$diagnostics=@{reads=0;chunked_reads=0;max_chunks=0}
try {
 for($i=0;$i -lt 1500;$i++){
  $raw=Get-Content -LiteralPath $path -Raw -ErrorAction Stop
  $joined=$raw -join ''
  if($raw -is [array]){$arrays++}
  if($raw.Length -lt $fixturePrefix.Length){
   $short++
   if($examples.Count -lt 5){$examples+=@{at=$i;raw_type=$raw.GetType().FullName;raw_length=$raw.Length;joined_characters=$joined.Length;prefix_matches=$joined.StartsWith($fixturePrefix,[StringComparison]::Ordinal)}}
  }
  $text=Read-ElyNativeLog $path $diagnostics
  if($text -isnot [string] -or -not$text.StartsWith($fixturePrefix,[StringComparison]::Ordinal)){$invalid++}
 }
} finally {$writer.GetAwaiter().GetResult()}
$bytes=[IO.File]::ReadAllBytes($path)
$expected=[Text.Encoding]::UTF8.GetBytes($fixturePrefix+($payload*30000))
$sha=[Security.Cryptography.SHA256]::Create()
try{$hash=[Convert]::ToHexString($sha.ComputeHash($bytes));$expectedHash=[Convert]::ToHexString($sha.ComputeHash($expected))}finally{$sha.Dispose()}
$verified=$invalid -eq 0 -and $hash -eq $expectedHash
@{reads=1500;legacy_array_reads=$arrays;legacy_short_length_reads=$short;reproduced=($short -gt 0);examples=$examples;normalized_invalid_reads=$invalid;diagnostics=$diagnostics;final_bytes=$bytes.Length;sha256=$hash;expected_sha256=$expectedHash;verified=$verified;ps=$PSVersionTable.PSVersion.ToString();test_sha256=(Get-FileHash -LiteralPath $PSCommandPath).Hash;reader_sha256=(Get-FileHash -LiteralPath "$PSScriptRoot/windows-native-log.ps1").Hash}|ConvertTo-Json -Depth 5|Set-Content -LiteralPath "$root/results.json" -Encoding utf8
Write-Output "$root/results.json"
Write-Output "Append check: legacy short-length reads=$short; normalized invalid reads=$invalid; exact final bytes=$($hash -eq $expectedHash)"
if(-not$verified){exit 1}
