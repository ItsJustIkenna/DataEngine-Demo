# PowerShell script to systematically convert all remaining tracing macros to UltraLogger
$files = @(
    "hostbuilder\src\handlers\models\kraken\balances_snapshot.rs",
    "hostbuilder\src\handlers\models\kraken\balances_update.rs",
    "hostbuilder\src\handlers\models\kraken\fast_parser.rs",
    "hostbuilder\src\handlers\models\kraken\level3_snapshot.rs",
    "hostbuilder\src\handlers\models\kraken\trade_snapshot.rs",
    "hostbuilder\src\handlers\models\kraken\trade_update.rs",
    "hostbuilder\src\handlers\websockets\kraken\kraken_websocket_handler.rs",
    "hostbuilder\src\api_endpoints.rs",
    "hostbuilder\src\caching.rs",
    "hostbuilder\src\data_pipeline.rs",
    "hostbuilder\src\kernel_bypass.rs",
    "hostbuilder\src\lib.rs",
    "hostbuilder\src\monitoring.rs",
    "hostbuilder\src\resilience.rs",
    "hostbuilder\src\security.rs",
    "hostbuilder\src\ultra_metrics.rs"
)

foreach ($file in $files) {
    if (Test-Path $file) {
        Write-Host "Converting $file..." -ForegroundColor Green
        $content = Get-Content $file -Raw
        
        # Replace tracing imports
        $content = $content -replace 'use tracing::\{[^}]*\};', 'use crate::logging_facade::MAIN_LOGGER; use crate::{log_info, log_warn, log_error, log_debug};'
        $content = $content -replace 'use tracing::[^;]*;', 'use crate::logging_facade::MAIN_LOGGER; use crate::{log_info, log_warn, log_error, log_debug};'
        
        # For Kraken models, use KRAKEN_LOGGER
        if ($file -like "*kraken*") {
            $content = $content -replace 'use crate::logging_facade::MAIN_LOGGER;', 'use crate::logging_facade::KRAKEN_LOGGER;'
            
            # Replace macro calls
            $content = $content -replace 'info!\(([^)]+)\)', 'log_info!(KRAKEN_LOGGER, $1)'
            $content = $content -replace 'warn!\(([^)]+)\)', 'log_warn!(KRAKEN_LOGGER, $1)'
            $content = $content -replace 'error!\(([^)]+)\)', 'log_error!(KRAKEN_LOGGER, $1)'
            $content = $content -replace 'debug!\(([^)]+)\)', 'log_debug!(KRAKEN_LOGGER, $1)'
        } else {
            # Replace macro calls
            $content = $content -replace 'info!\(([^)]+)\)', 'log_info!(MAIN_LOGGER, $1)'
            $content = $content -replace 'warn!\(([^)]+)\)', 'log_warn!(MAIN_LOGGER, $1)'
            $content = $content -replace 'error!\(([^)]+)\)', 'log_error!(MAIN_LOGGER, $1)'
            $content = $content -replace 'debug!\(([^)]+)\)', 'log_debug!(MAIN_LOGGER, $1)'
        }
        
        Set-Content $file $content
        Write-Host "✅ Converted $file" -ForegroundColor Cyan
    } else {
        Write-Host "❌ File not found: $file" -ForegroundColor Red
    }
}

Write-Host "`n🎉 All tracing macro conversions completed!" -ForegroundColor Green
