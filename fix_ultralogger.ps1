# PowerShell script to fix all remaining UltraLogger conversion issues
Write-Host "🔧 Fixing UltraLogger conversion issues..." -ForegroundColor Yellow

# Files with malformed log_log_* calls
$malformed_files = @(
    "hostbuilder\src\handlers\models\kraken\level3_snapshot.rs",
    "hostbuilder\src\handlers\models\kraken\balances_snapshot.rs"
)

Write-Host "📝 Fixing malformed macro calls..." -ForegroundColor Cyan
foreach ($file in $malformed_files) {
    if (Test-Path $file) {
        $content = Get-Content $file -Raw
        $content = $content -replace 'log_log_debug!', 'log_debug!'
        $content = $content -replace 'log_log_info!', 'log_info!'
        $content = $content -replace 'log_log_warn!', 'log_warn!'
        $content = $content -replace 'log_log_error!', 'log_error!'
        Set-Content $file $content
        Write-Host "  ✅ Fixed malformed calls in $file" -ForegroundColor Green
    }
}

# Files with tracing:: prefixed calls
$tracing_prefix_files = @(
    "hostbuilder\src\handlers\websockets\kraken\kraken_websocket_handler.rs",
    "hostbuilder\src\lib.rs"
)

Write-Host "📝 Fixing tracing:: prefixed calls..." -ForegroundColor Cyan
foreach ($file in $tracing_prefix_files) {
    if (Test-Path $file) {
        $content = Get-Content $file -Raw
        $content = $content -replace 'tracing::log_debug!', 'log_debug!'
        $content = $content -replace 'tracing::log_info!', 'log_info!'
        $content = $content -replace 'tracing::log_warn!', 'log_warn!'
        $content = $content -replace 'tracing::log_error!', 'log_error!'
        Set-Content $file $content
        Write-Host "  ✅ Fixed tracing:: prefixes in $file" -ForegroundColor Green
    }
}

# Fix import issues - replace wrong imports with correct crate-level imports
$files_with_import_issues = @(
    "hostbuilder\src\handlers\models\kraken\level3_snapshot.rs",
    "hostbuilder\src\handlers\models\kraken\balances_snapshot.rs",
    "hostbuilder\src\handlers\models\kraken\balances_update.rs",
    "hostbuilder\src\handlers\models\kraken\fast_parser.rs",
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

Write-Host "📝 Fixing import statements..." -ForegroundColor Cyan
foreach ($file in $files_with_import_issues) {
    if (Test-Path $file) {
        $content = Get-Content $file -Raw
        
        # Remove old tracing imports and wrong facade imports
        $content = $content -replace 'use crate::logging_facade::\{[^}]*\};', ''
        $content = $content -replace 'use crate::\{[^}]*log_[^}]*\};', ''
        $content = $content -replace 'use tracing::\{[^}]*\};', ''
        
        # For Kraken files, add proper imports
        if ($file -like "*kraken*") {
            if ($content -notmatch 'use crate::logging_facade::KRAKEN_LOGGER;') {
                $content = $content -replace '(use [^;]+;[\r\n]*)', "`$1use crate::logging_facade::KRAKEN_LOGGER;`nuse crate::{log_debug, log_info, log_warn, log_error};`n"
            }
        } else {
            # For other files, add MAIN_LOGGER imports
            if ($content -notmatch 'use crate::logging_facade::MAIN_LOGGER;') {
                $content = $content -replace '(use [^;]+;[\r\n]*)', "`$1use crate::logging_facade::MAIN_LOGGER;`nuse crate::{log_debug, log_info, log_warn, log_error};`n"
            }
        }
        
        # Fix trace! calls in caching.rs
        if ($file -like "*caching.rs*") {
            $content = $content -replace 'trace!', 'log_debug!'
            $content = $content -replace '\$logger:expr,', 'MAIN_LOGGER,'
        }
        
        Set-Content $file $content
        Write-Host "  ✅ Fixed imports in $file" -ForegroundColor Green
    }
}

Write-Host "`n🎉 All UltraLogger conversion fixes completed!" -ForegroundColor Green
