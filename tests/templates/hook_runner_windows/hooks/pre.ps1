$inputJson = Get-Content -Raw
$data = $inputJson | ConvertFrom-Json
$outPath = Join-Path $data.output_dir "pre-hook.txt"
"pre hook executed via powershell runner" | Set-Content -Path $outPath -NoNewline
