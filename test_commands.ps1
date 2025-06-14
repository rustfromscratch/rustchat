# Test the new command system
$process = Start-Process -FilePath "cargo" -ArgumentList "run", "--bin", "rustchat-cli" -WorkingDirectory "c:\Users\w33d\Documents\GitHub\rustchat" -PassThru -NoNewWindow -RedirectStandardInput

# Wait for connection
Start-Sleep -Seconds 3

# Test help command
"echo '/help' | Write-Host"
"/help" | Out-File -Encoding utf8 temp_input.txt
Get-Content temp_input.txt | Out-File -Encoding utf8 -Append temp_input.txt

# Test quit command
"/quit" | Out-File -Encoding utf8 -Append temp_input.txt

# Send input to process
Get-Content temp_input.txt | ForEach-Object { $process.StandardInput.WriteLine($_) }

# Wait and cleanup
Start-Sleep -Seconds 2
$process.Kill()
Remove-Item temp_input.txt -ErrorAction SilentlyContinue
