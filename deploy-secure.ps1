# DataEngine Secure Deployment Script
# This script allows secure deployment without exposing credentials in Git

param(
    [Parameter(Mandatory=$false)]
    [string]$Environment = "dev",
    
    [Parameter(Mandatory=$false)]
    [string]$ReleaseName = "data-engine-$Environment",
    
    [Parameter(Mandatory=$false)]
    [string]$Namespace = "default",
    
    [Parameter(Mandatory=$false)]
    [switch]$UseExternalSecrets = $false
)

Write-Host "🚀 DataEngine Secure Deployment Script" -ForegroundColor Green
Write-Host "Environment: $Environment" -ForegroundColor Yellow
Write-Host "Release Name: $ReleaseName" -ForegroundColor Yellow
Write-Host "Namespace: $Namespace" -ForegroundColor Yellow

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$helmDir = Join-Path $scriptDir "k8s\helm"
$valuesFile = Join-Path $helmDir "values.yaml"

# Check if Helm chart exists
if (-not (Test-Path $helmDir)) {
    Write-Error "❌ Helm chart directory not found: $helmDir"
    exit 1
}

if (-not (Test-Path $valuesFile)) {
    Write-Error "❌ Main values.yaml not found: $valuesFile"
    exit 1
}

Write-Host "`n🔐 Credential Input (secure prompting)" -ForegroundColor Cyan

if ($UseExternalSecrets) {
    Write-Host "✅ Using external secret management - no credential input needed" -ForegroundColor Green
    $helmArgs = @(
        "install", $ReleaseName, $helmDir,
        "--namespace", $Namespace,
        "--create-namespace",
        "-f", $valuesFile,
        "--set", "externalSecrets.enabled=true",
        "--set", "global.environment=$Environment"
    )
} else {
    # Prompt for credentials securely
    Write-Host "Enter database credentials:" -ForegroundColor Yellow
    $dbUsername = Read-Host "Database Username"
    $dbPasswordSecure = Read-Host "Database Password" -AsSecureString
    $dbPassword = [System.Runtime.InteropServices.Marshal]::PtrToStringAuto([System.Runtime.InteropServices.Marshal]::SecureStringToBSTR($dbPasswordSecure))
    
    Write-Host "Enter Redis credentials:" -ForegroundColor Yellow
    $redisPasswordSecure = Read-Host "Redis Password" -AsSecureString
    $redisPassword = [System.Runtime.InteropServices.Marshal]::PtrToStringAuto([System.Runtime.InteropServices.Marshal]::SecureStringToBSTR($redisPasswordSecure))
    
    Write-Host "Enter API credentials:" -ForegroundColor Yellow
    $krakenApiKey = Read-Host "Kraken API Key"
    
    # Build Helm command with secure credential injection
    $helmArgs = @(
        "install", $ReleaseName, $helmDir,
        "--namespace", $Namespace,
        "--create-namespace",
        "-f", $valuesFile,
        "--set-string", "secrets.database.username=$dbUsername",
        "--set-string", "secrets.database.password=$dbPassword",
        "--set-string", "secrets.redis.password=$redisPassword",
        "--set-string", "secrets.api_keys.kraken=$krakenApiKey",
        "--set-string", "postgresql.auth.username=$dbUsername",
        "--set-string", "postgresql.auth.password=$dbPassword",
        "--set-string", "postgresql.auth.postgresPassword=$dbPassword",
        "--set-string", "redis.auth.password=$redisPassword",
        "--set", "global.environment=$Environment"
    )
    
    # Clear sensitive variables
    $dbPassword = $null
    $redisPassword = $null
    $dbPasswordSecure.Dispose()
    $redisPasswordSecure.Dispose()
}

Write-Host "`n🚀 Deploying DataEngine..." -ForegroundColor Green
Write-Host "Command: helm $($helmArgs -join ' ')" -ForegroundColor Gray

try {
    & helm $helmArgs
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "`n✅ DataEngine deployed successfully!" -ForegroundColor Green
        Write-Host "`nTo check status:" -ForegroundColor Yellow
        Write-Host "  helm status $ReleaseName -n $Namespace" -ForegroundColor White
        Write-Host "  kubectl get pods -n $Namespace -l app.kubernetes.io/name=data-engine" -ForegroundColor White
        
        Write-Host "`nTo view logs:" -ForegroundColor Yellow
        Write-Host "  kubectl logs -n $Namespace -l app.kubernetes.io/name=data-engine -f" -ForegroundColor White
        
        Write-Host "`nTo upgrade:" -ForegroundColor Yellow
        Write-Host "  helm upgrade $ReleaseName $helmDir -n $Namespace [--set-string options]" -ForegroundColor White
    } else {
        Write-Error "❌ Deployment failed with exit code $LASTEXITCODE"
        exit 1
    }
} catch {
    Write-Error "❌ Deployment failed: $($_.Exception.Message)"
    exit 1
}

Write-Host "`n🔒 Security Notes:" -ForegroundColor Magenta
Write-Host "  • Credentials were injected securely via --set-string" -ForegroundColor White
Write-Host "  • No credentials were stored in files or Git history" -ForegroundColor White  
Write-Host "  • Use external secret management for production" -ForegroundColor White
Write-Host "  • Rotate credentials regularly" -ForegroundColor White

# Usage examples:
# .\deploy-secure.ps1                                    # Dev deployment with credential prompts
# .\deploy-secure.ps1 -Environment prod                  # Prod deployment with credential prompts
# .\deploy-secure.ps1 -UseExternalSecrets               # Deployment with external secret management
# .\deploy-secure.ps1 -ReleaseName my-data-engine       # Custom release name
