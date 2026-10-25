function Read-ElyOfficePart([string]$Path,[string]$Part) {
 $zip=[IO.Compression.ZipFile]::OpenRead($Path)
 try{
  $entry=$zip.GetEntry($Part)
  if(-not $entry){throw "Missing document part: $Part"}
  $reader=[IO.StreamReader]::new($entry.Open())
  try{return [xml]$reader.ReadToEnd()}finally{$reader.Dispose()}
 }finally{$zip.Dispose()}
}
function Read-ElyTableCell([string]$Path,[ValidateSet('A1','A2','B1')][string]$Address,[bool]$OpenDocument) {
 if(-not $OpenDocument){
  $sheet=Read-ElyOfficePart $Path 'xl/worksheets/sheet1.xml'
  $cell=$sheet.SelectSingleNode("//*[local-name()='c' and @r='$Address']")
  if(-not $cell){throw "Missing cell: $Address"}
  $value=$cell.SelectSingleNode('./*[local-name()="v"]')
  $formula=$cell.SelectSingleNode('./*[local-name()="f"]')
  $text=if($value){$value.InnerText}else{''}
  if($cell.GetAttribute('t') -eq 's'){
   $strings=Read-ElyOfficePart $Path 'xl/sharedStrings.xml'
   $items=$strings.SelectNodes('/*/*[local-name()="si"]')
   $index=0
   if(-not [int]::TryParse($text,[ref]$index) -or $index -lt 0 -or $index -ge $items.Count){throw 'Invalid shared-string index'}
   $text=(@($items[$index].SelectNodes('.//*[local-name()="t"]')|ForEach-Object {$_.InnerText}) -join '')
  }elseif($cell.GetAttribute('t') -eq 'inlineStr'){
   $text=(@($cell.SelectNodes('./*[local-name()="is"]//*[local-name()="t"]')|ForEach-Object {$_.InnerText}) -join '')
  }
  return [pscustomobject]@{Text=$text;Formula=$(if($formula){$formula.InnerText}else{''});Value=$(if($value){$value.InnerText}else{''})}
 }
 $xml=Read-ElyOfficePart $Path 'content.xml'
 $tableNs='urn:oasis:names:tc:opendocument:xmlns:table:1.0'
 $officeNs='urn:oasis:names:tc:opendocument:xmlns:office:1.0'
 $column=if($Address -eq 'B1'){1}else{0}
 $row=if($Address -eq 'A2'){1}else{0}
 $table=$xml.SelectSingleNode('//*[local-name()="spreadsheet"]/*[local-name()="table"]')
 if(-not $table){throw 'Missing spreadsheet table'}
 $rowIndex=0
 foreach($line in $table.SelectNodes('./*[local-name()="table-row"]')){
  $repeat=$line.GetAttribute('number-rows-repeated',$tableNs)
  $rows=if($repeat){[long]$repeat}else{1}
  if($rows -lt 1){throw 'Invalid row repeat'}
  if($row -ge $rowIndex -and $row -lt $rowIndex+$rows){
   $columnIndex=0
   foreach($cell in $line.SelectNodes('./*[local-name()="table-cell" or local-name()="covered-table-cell"]')){
    $repeat=$cell.GetAttribute('number-columns-repeated',$tableNs)
    $columns=if($repeat){[long]$repeat}else{1}
    if($columns -lt 1){throw 'Invalid column repeat'}
    if($column -ge $columnIndex -and $column -lt $columnIndex+$columns){
     $text=(@($cell.SelectNodes('./*[local-name()="p"]')|ForEach-Object {$_.InnerText}) -join "`n")
     return [pscustomobject]@{Text=$text;Formula=$cell.GetAttribute('formula',$tableNs);Value=$cell.GetAttribute('value',$officeNs)}
    }
    $columnIndex+=$columns
   }
   break
  }
  $rowIndex+=$rows
 }
 throw "Missing OpenDocument cell: $Address"
}
function Assert-ElyOfficeContent([string]$Path,[string]$Application,[string]$Marker,[string]$Second,[bool]$AfterCancel,[bool]$OpenDocument) {
 if($Application -eq 'show'){
  $part=if($OpenDocument){'content.xml'}else{'ppt/presentation.xml'}
  $xml=Read-ElyOfficePart $Path $part
  $query=if($OpenDocument){'//*[local-name()="page" and namespace-uri()="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"]'}else{'//*[local-name()="sldIdLst"]/*[local-name()="sldId"]'}
  $wanted=if($AfterCancel){3}else{2}
  if($xml.SelectNodes($query).Count -ne $wanted){throw 'Saved slide count differs'}
  return
 }
 $a1=Read-ElyTableCell $Path 'A1' $OpenDocument
 $b1=Read-ElyTableCell $Path 'B1' $OpenDocument
 $formula=if($OpenDocument){'of:=1+2'}else{'1+2'}
 if($a1.Text -cne $Marker){throw 'Cell A1 lost its Unicode text'}
 if($b1.Formula -cne $formula -or $b1.Value -cne '3'){throw 'Cell B1 formula or its own cached value differs'}
 if($AfterCancel -and (Read-ElyTableCell $Path 'A2' $OpenDocument).Text -cne $Second){throw 'Cell A2 lost the draft after Cancel'}
}
function Test-ElyOpenedInput([string]$Text,[string]$Path) {
 $name=[regex]::Escape([IO.Path]::GetFileName($Path))
 $opened=[regex]::Match($Text,"opened $name as tab \d+, risk ")
 return $opened.Success -and $Text.LastIndexOf('first tile drawn') -gt $opened.Index
}
