$inputJson = Get-Content -Raw
$data = $inputJson | ConvertFrom-Json
$outPath = Join-Path $data.output_dir "post-hook.txt"
"post hook executed via powershell runner" | Set-Content -Path $outPath -NoNewline
