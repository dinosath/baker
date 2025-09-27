$inputJson = [Console]::In.ReadToEnd()
if (-not $inputJson) {
    throw "Hook did not receive any input"
}
$data = $inputJson | ConvertFrom-Json
$outPath = Join-Path $data.output_dir "pre-hook.txt"
[IO.File]::WriteAllText($outPath, "pre hook executed via powershell runner`r`n")
