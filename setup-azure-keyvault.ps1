# Azure Key Vault Setup Script for DataEngine
# This script creates a Key Vault and populates it with all required secrets

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Azure Key Vault Setup for DataEngine" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Step 1: Login to Azure
Write-Host "Step 1: Logging into Azure..." -ForegroundColor Yellow
az login

if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to login to Azure"
    exit 1
}

# Step 2: Get subscription ID
Write-Host "`nStep 2: Getting subscription ID..." -ForegroundColor Yellow
$subscriptionId = az account show --query id -o tsv
Write-Host "Subscription ID: $subscriptionId" -ForegroundColor Green

# Step 3: Set variables
$keyVaultName = Read-Host "`nEnter a unique Key Vault name (e.g., dataengine-kv-prod)"
$resourceGroup = Read-Host "Enter resource group name (or leave empty to create new 'dataengine-rg')"
$location = Read-Host "Enter Azure region (e.g., eastus, westus2, or leave empty for 'eastus')"

if ([string]::IsNullOrWhiteSpace($resourceGroup)) {
    $resourceGroup = "dataengine-rg"
}

if ([string]::IsNullOrWhiteSpace($location)) {
    $location = "eastus"
}

Write-Host "`nConfiguration:" -ForegroundColor Cyan
Write-Host "  Key Vault Name: $keyVaultName" -ForegroundColor White
Write-Host "  Resource Group: $resourceGroup" -ForegroundColor White
Write-Host "  Location: $location" -ForegroundColor White
Write-Host ""

$confirm = Read-Host "Continue with this configuration? (y/n)"
if ($confirm -ne 'y') {
    Write-Host "Setup cancelled" -ForegroundColor Red
    exit 0
}

# Step 4: Create resource group if it doesn't exist
Write-Host "`nStep 4: Creating resource group (if needed)..." -ForegroundColor Yellow
az group create --name $resourceGroup --location $location

# Step 5: Create Key Vault
Write-Host "`nStep 5: Creating Key Vault..." -ForegroundColor Yellow
az keyvault create `
    --name $keyVaultName `
    --resource-group $resourceGroup `
    --location $location `
    --enable-rbac-authorization false

if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to create Key Vault"
    exit 1
}

Write-Host "Key Vault created successfully!" -ForegroundColor Green

# Step 6: Create Service Principal
Write-Host "`nStep 6: Creating Service Principal for GitHub Actions..." -ForegroundColor Yellow
$spOutput = az ad sp create-for-rbac `
    --name "DataEngineGitHubActions-$(Get-Date -Format 'yyyyMMdd')" `
    --role contributor `
    --scopes "/subscriptions/$subscriptionId" `
    --json-auth

if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to create Service Principal"
    exit 1
}

Write-Host "`n========================================" -ForegroundColor Green
Write-Host "SERVICE PRINCIPAL CREDENTIALS (COPY THIS TO GITHUB SECRET 'AZURE_CREDENTIALS'):" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host $spOutput -ForegroundColor White
Write-Host "========================================" -ForegroundColor Green
Write-Host ""

# Parse the service principal output to get clientId
$spJson = $spOutput | ConvertFrom-Json
$clientId = $spJson.clientId

# Step 7: Grant Key Vault permissions to Service Principal
Write-Host "`nStep 7: Granting Key Vault permissions to Service Principal..." -ForegroundColor Yellow
az keyvault set-policy `
    --name $keyVaultName `
    --spn $clientId `
    --secret-permissions get list

Write-Host "Permissions granted successfully!" -ForegroundColor Green

# Step 8: Collect secrets
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Step 8: Collecting Secret Values" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Please provide the following secret values:" -ForegroundColor Yellow
Write-Host "(Press Enter to skip optional secrets)" -ForegroundColor Gray
Write-Host ""

$secrets = @{}

# PostgreSQL secrets
Write-Host "PostgreSQL Database:" -ForegroundColor Cyan
$secrets["POSTGRES-HOST"] = Read-Host "  POSTGRES-HOST (e.g., mydb.postgres.database.azure.com)"
$secrets["POSTGRES-PORT"] = Read-Host "  POSTGRES-PORT (default: 5432)"
if ([string]::IsNullOrWhiteSpace($secrets["POSTGRES-PORT"])) { $secrets["POSTGRES-PORT"] = "5432" }
$secrets["POSTGRES-USER"] = Read-Host "  POSTGRES-USER (e.g., admin@mydb)"
$secrets["POSTGRES-PASSWORD"] = Read-Host "  POSTGRES-PASSWORD" -AsSecureString
$secrets["POSTGRES-PASSWORD"] = [Runtime.InteropServices.Marshal]::PtrToStringAuto([Runtime.InteropServices.Marshal]::SecureStringToBSTR($secrets["POSTGRES-PASSWORD"]))
$secrets["POSTGRES-DB"] = Read-Host "  POSTGRES-DB (database name)"
Write-Host ""

# Redis secrets
Write-Host "Redis Cache:" -ForegroundColor Cyan
$secrets["REDIS-HOST"] = Read-Host "  REDIS-HOST (e.g., mycache.redis.cache.windows.net)"
$secrets["REDIS-PORT"] = Read-Host "  REDIS-PORT (default: 6380)"
if ([string]::IsNullOrWhiteSpace($secrets["REDIS-PORT"])) { $secrets["REDIS-PORT"] = "6380" }
$secrets["REDIS-PASSWORD"] = Read-Host "  REDIS-PASSWORD" -AsSecureString
$secrets["REDIS-PASSWORD"] = [Runtime.InteropServices.Marshal]::PtrToStringAuto([Runtime.InteropServices.Marshal]::SecureStringToBSTR($secrets["REDIS-PASSWORD"]))
Write-Host ""

# Kraken API secrets
Write-Host "Kraken API:" -ForegroundColor Cyan
$secrets["KRAKEN-API-KEY"] = Read-Host "  KRAKEN-API-KEY"
$secrets["KRAKEN-SECRET-KEY"] = Read-Host "  KRAKEN-SECRET-KEY" -AsSecureString
$secrets["KRAKEN-SECRET-KEY"] = [Runtime.InteropServices.Marshal]::PtrToStringAuto([Runtime.InteropServices.Marshal]::SecureStringToBSTR($secrets["KRAKEN-SECRET-KEY"]))
Write-Host ""

# Elasticsearch secrets
Write-Host "Elasticsearch Cloud:" -ForegroundColor Cyan
$secrets["ELASTIC-CLOUD-ENDPOINT"] = Read-Host "  ELASTIC-CLOUD-ENDPOINT (e.g., https://xxx.es.us-central1.gcp.cloud.es.io)"
$secrets["ELASTIC-CLOUD-USERNAME"] = Read-Host "  ELASTIC-CLOUD-USERNAME (default: elastic)"
if ([string]::IsNullOrWhiteSpace($secrets["ELASTIC-CLOUD-USERNAME"])) { $secrets["ELASTIC-CLOUD-USERNAME"] = "elastic" }
$secrets["ELASTIC-CLOUD-PASSWORD"] = Read-Host "  ELASTIC-CLOUD-PASSWORD" -AsSecureString
$secrets["ELASTIC-CLOUD-PASSWORD"] = [Runtime.InteropServices.Marshal]::PtrToStringAuto([Runtime.InteropServices.Marshal]::SecureStringToBSTR($secrets["ELASTIC-CLOUD-PASSWORD"]))
Write-Host ""

# Step 9: Upload secrets to Key Vault
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Step 9: Uploading Secrets to Key Vault" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$successCount = 0
$failCount = 0

foreach ($secretName in $secrets.Keys) {
    $secretValue = $secrets[$secretName]
    
    if ([string]::IsNullOrWhiteSpace($secretValue)) {
        Write-Host "  [SKIP] $secretName (empty)" -ForegroundColor Gray
        continue
    }
    
    try {
        az keyvault secret set `
            --vault-name $keyVaultName `
            --name $secretName `
            --value $secretValue `
            --output none
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  [OK] $secretName" -ForegroundColor Green
            $successCount++
        } else {
            Write-Host "  [FAIL] $secretName" -ForegroundColor Red
            $failCount++
        }
    } catch {
        Write-Host "  [FAIL] $secretName - $_" -ForegroundColor Red
        $failCount++
    }
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Summary:" -ForegroundColor Cyan
Write-Host "  Secrets uploaded: $successCount" -ForegroundColor Green
Write-Host "  Failed: $failCount" -ForegroundColor $(if ($failCount -gt 0) { "Red" } else { "Green" })
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Step 10: Final instructions
Write-Host "========================================" -ForegroundColor Yellow
Write-Host "NEXT STEPS - GitHub Secrets Setup" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Yellow
Write-Host ""
Write-Host "1. Go to: https://github.com/Nwagbara-Group-LLC/DataEngine/settings/secrets/actions" -ForegroundColor White
Write-Host ""
Write-Host "2. Create/Update these secrets:" -ForegroundColor White
Write-Host ""
Write-Host "   Secret Name: AZURE_CREDENTIALS" -ForegroundColor Cyan
Write-Host "   Value: (Copy the JSON from Step 6 above)" -ForegroundColor White
Write-Host ""
Write-Host "   Secret Name: AZURE_KEYVAULT_NAME" -ForegroundColor Cyan
Write-Host "   Value: $keyVaultName" -ForegroundColor White
Write-Host ""
Write-Host "3. Re-run your GitHub Actions workflow" -ForegroundColor White
Write-Host ""
Write-Host "========================================" -ForegroundColor Yellow
Write-Host "Setup Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Yellow
