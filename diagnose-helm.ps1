# Diagnostic script to understand why Helm deployment fails
# Run this on the remote runner

Write-Host "=== HELM DIAGNOSTIC ===" -ForegroundColor Cyan

# 1. Check what Helm THINKS it deployed
Write-Host "`n1. Helm Manifest (what Helm generated):" -ForegroundColor Yellow
$manifest = helm get manifest dataengine -n dataengine 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "Manifest exists. Resource counts:" -ForegroundColor Green
    ($manifest | Select-String -Pattern "^kind:" -AllMatches).Matches | Group-Object Value | Format-Table Count, Name
} else {
    Write-Host "No manifest found: $manifest" -ForegroundColor Red
}

# 2. Check what actually EXISTS in Kubernetes
Write-Host "`n2. Actual Kubernetes Resources:" -ForegroundColor Yellow
kubectl get all,serviceaccounts,statefulsets,pvc -n dataengine

# 3. Check Helm release status
Write-Host "`n3. Helm Release Status:" -ForegroundColor Yellow
helm status dataengine -n dataengine 2>&1

# 4. Check for recent events (errors)
Write-Host "`n4. Recent Kubernetes Events:" -ForegroundColor Yellow
kubectl get events -n dataengine --sort-by='.lastTimestamp' | Select-Object -Last 20

# 5. Check if ServiceAccount exists
Write-Host "`n5. ServiceAccount Status:" -ForegroundColor Yellow
kubectl get serviceaccount -n dataengine

# 6. If ServiceAccount exists, check its annotations
$sa = kubectl get serviceaccount dataengine-data-engine -n dataengine -o json 2>&1 | ConvertFrom-Json
if ($sa) {
    Write-Host "`nServiceAccount Annotations:" -ForegroundColor Cyan
    $sa.metadata.annotations | ConvertTo-Json
}

# 7. Check Helm values that were actually used
Write-Host "`n7. Helm Values Used:" -ForegroundColor Yellow
helm get values dataengine -n dataengine

Write-Host "`n=== END DIAGNOSTIC ===" -ForegroundColor Cyan
