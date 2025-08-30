# PowerShell script to fix logging macros in main.rs
$file = "program\src\main.rs"
$content = Get-Content $file -Raw

# Replace simple info! calls
$content = $content -replace 'info!\("([^"]*)"(\))', 'log_info!(MAIN_LOGGER, "$1"$2'

# Replace info! calls with format arguments
$content = $content -replace 'info!\("([^"]*)",([^)]*)\)', 'log_info!(MAIN_LOGGER, "$1",$2)'

# Replace simple warn! calls  
$content = $content -replace 'warn!\("([^"]*)"(\))', 'log_warn!(MAIN_LOGGER, "$1"$2'

# Replace warn! calls with format arguments
$content = $content -replace 'warn!\("([^"]*)",([^)]*)\)', 'log_warn!(MAIN_LOGGER, "$1",$2)'

# Save the file
Set-Content $file $content

Write-Host "Logging macros replacement completed"
