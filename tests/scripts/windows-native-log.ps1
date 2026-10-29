function Read-ElyNativeLog {
 param([Parameter(Mandatory)][string]$Path,[Collections.IDictionary]$Diagnostics)
 if(-not(Test-Path -LiteralPath $Path)){return ''}
 $parts=@(Get-Content -LiteralPath $Path -Raw -ErrorAction Stop)
 if($null -ne $Diagnostics){
  $Diagnostics.reads++
  if($parts.Count -gt 1){$Diagnostics.chunked_reads++}
  $Diagnostics.max_chunks=[Math]::Max($Diagnostics.max_chunks,$parts.Count)
 }
 return $parts -join ''
}
