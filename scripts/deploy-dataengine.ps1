#!/usr/bin/env pwsh

<#
.SYNOPSIS
    Deploy DataEngine application in database-only mode

.DESCRIPTION
    This script deploys the DataEngine application configured for database-only mode,
    where it persists websocket data to database and cache without message broker functionality.

.PARAMETER ReleaseName
    Name of the Helm release (default: data-engine)

.PARAMETER Namespace  
    Kubernetes namespace (default: data-engine)

.PARAMETER ValuesFile
    Custom values file (default: k8s/data-engine-helm/values.yaml)

.PARAMETER DatabaseOnly
    Enable database-only mode (default: true)

.PARAMETER Upgrade
    Perform upgrade instead of install

.PARAMETER DryRun
    Perform a dry run without actually deploying

.EXAMPLE
    .\deploy-dataengine.ps1
    .\deploy-dataengine.ps1 -ReleaseName "my-dataengine" -Namespace "production"
    .\deploy-dataengine.ps1 -DryRun
#>

param(
    [string]$ReleaseName = "data-engine",
    [string]$Namespace = "data-engine", 
    [string]$ValuesFile = "",
    [bool]$DatabaseOnly = $true,
    [switch]$Upgrade,
    [switch]$DryRun
)

# Helper Functions
function Write-Header {
    param([string]$Message)
    Write-Host ""
    Write-Host "=" * 50 -ForegroundColor Cyan
    Write-Host $Message -ForegroundColor Yellow
    Write-Host "=" * 50 -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host $Message -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message) 
    Write-Host $Message -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host $Message -ForegroundColor Red
}

function Test-Command {
    param([string]$Command)
    return Get-Command $Command -ErrorAction SilentlyContinue
}

function Wait-ForPods {
    param([string]$Namespace, [string]$ReleaseName)
    
    Write-Host "Waiting for DataEngine pods to be ready..."
    $timeout = 300 # 5 minutes
    $elapsed = 0
    
    do {
        $pods = kubectl get pods -n $Namespace -l "app.kubernetes.io/instance=$ReleaseName" --no-headers 2>$null
        if ($pods) {
            $readyPods = $pods | Where-Object { $_ -match "Running|Completed" }
            $totalPods = ($pods | Measure-Object).Count
            $readyCount = ($readyPods | Measure-Object).Count
            
            if ($readyCount -eq $totalPods -and $totalPods -gt 0) {
                Write-Success "All DataEngine pods are ready!"
                return $true
            }
            
            Write-Host "Pods ready: $readyCount/$totalPods"
        }
        
        Start-Sleep 10
        $elapsed += 10
    } while ($elapsed -lt $timeout)
    
    Write-Warning "Timeout waiting for pods to be ready"
    return $false
}

function Show-DataEngineAccess {
    param([string]$Namespace, [string]$ReleaseName)
    
    Write-Success "DataEngine is now running in database-only mode!"
    Write-Host ""
    
    # Show pod information
    $pods = kubectl get pods -n $Namespace -l "app.kubernetes.io/instance=$ReleaseName" --no-headers 2>$null
    if ($pods) {
        Write-Host "DataEngine Pods:"
        kubectl get pods -n $Namespace -l "app.kubernetes.io/instance=$ReleaseName"
        Write-Host ""
    }
    
    Write-Host "Access Information:"
    Write-Host "=================="
    
    Write-Host "Check DataEngine logs:"
    Write-Host "   kubectl logs -f deployment/$ReleaseName -n $Namespace"
    
    Write-Host "Check pod status:"
    Write-Host "   kubectl get pods -n $Namespace -l app.kubernetes.io/instance=$ReleaseName"
    
    Write-Host "Port forward for API access:"
    Write-Host "   kubectl port-forward service/$ReleaseName 8080:8080 -n $Namespace"
    
    Write-Host "Check configuration:"
    Write-Host "   kubectl get configmap $ReleaseName-config -n $Namespace -o yaml"
    
    Write-Host "Check environment:"
    Write-Host "   kubectl exec deployment/$ReleaseName -n $Namespace -- env | grep DATABASE"
    Write-Host ""
    
    Write-Host "To cleanup:"
    Write-Host "helm uninstall $ReleaseName -n $Namespace"
}

# Read credentials from .env file
function Get-EnvCredentials {
    $envFile = "scripts/.env"
    $credentials = @{}
    
    if (Test-Path $envFile) {
        Write-Host "Reading credentials from $envFile..."
        Get-Content $envFile | ForEach-Object {
            if ($_ -match "^([^#][^=]+)=(.+)$") {
                $credentials[$matches[1]] = $matches[2]
            }
        }
    } else {
        Write-Warning "No .env file found. Using placeholder credentials."
        $credentials = @{
            "DATABASE_URL" = "placeholder"
            "REDIS_URL" = "placeholder"
            "REDIS_PASSWORD" = "placeholder"
        }
    }
    
    return $credentials
}

# Main Deployment Function
function Deploy-DataEngine {
    param([string]$ReleaseName, [string]$Namespace, [string]$ValuesFile, [bool]$IsUpgrade, [bool]$IsDryRun, [bool]$DatabaseOnlyMode)
    
    try {
        # Auto-select values file if not specified
        if ([string]::IsNullOrEmpty($ValuesFile)) {
            if ($DatabaseOnlyMode) {
                $ValuesFile = "k8s/data-engine-helm/values-database-only.yaml"
            } else {
                $ValuesFile = "k8s/data-engine-helm/values.yaml"
            }
        }
        
        # Validate prerequisites
        if (-not (Test-Command "kubectl")) {
            Write-Error "kubectl is not installed or not in PATH"
            exit 1
        }
        
        if (-not (Test-Command "helm")) {
            Write-Error "helm is not installed or not in PATH"
            exit 1
        }
        
        # Check if values file exists
        if (-not (Test-Path $ValuesFile)) {
            Write-Error "Values file not found: $ValuesFile"
            exit 1
        }
        
        # Create namespace if it doesn't exist
        $namespaceExists = kubectl get namespace $Namespace 2>$null
        if (-not $namespaceExists) {
            Write-Host "Creating namespace: $Namespace"
            kubectl create namespace $Namespace
        }
        
        # Get credentials
        $creds = Get-EnvCredentials
        
        # Build Helm command
        $helmCommand = @("helm")
        
        if ($IsUpgrade) {
            $helmCommand += "upgrade", $ReleaseName
        } else {
            $helmCommand += "install", $ReleaseName
        }
        
        $helmCommand += "k8s/data-engine-helm"
        $helmCommand += "--namespace", $Namespace
        $helmCommand += "--values", $ValuesFile
        $helmCommand += "--create-namespace"
        
        # Only add credential overrides, no configuration overrides since values file handles that
        # External database configuration
        if ($creds["DATABASE_URL"] -ne "placeholder") {
            $helmCommand += "--set", "secrets.database.url=$($creds['DATABASE_URL'])"
        }
        if ($creds["POSTGRES_PASSWORD"] -ne "placeholder") {
            $helmCommand += "--set", "secrets.database.password=$($creds['POSTGRES_PASSWORD'])"
        }
        
        # External Redis configuration  
        if ($creds["REDIS_URL"] -ne "placeholder") {
            $helmCommand += "--set", "secrets.redis.password=$($creds['REDIS_PASSWORD'])"
            $helmCommand += "--set", "secrets.redis.url=$($creds['REDIS_URL'])"
        }
        
        # API keys
        $krakenApiKey = if ($creds["KRAKEN_API_KEY"]) { $creds["KRAKEN_API_KEY"] } else { "placeholder" }
        $krakenSecretKey = if ($creds["KRAKEN_SECRET_KEY"]) { $creds["KRAKEN_SECRET_KEY"] } else { "placeholder" }
        $helmCommand += "--set", "secrets.api_keys.kraken_api_key=$krakenApiKey"
        $helmCommand += "--set", "secrets.api_keys.kraken_secret_key=$krakenSecretKey"
        
        if ($IsDryRun) {
            $helmCommand += "--dry-run"
        }
        
        if ($IsUpgrade) {
            $helmCommand += "--install" # Install if doesn't exist
        }
        
        Write-Host "Executing: $($helmCommand -join ' ')"
        Write-Host ""
        
        # Execute Helm command
        & $helmCommand[0] $helmCommand[1..($helmCommand.Count-1)]
        
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Helm deployment failed with exit code: $LASTEXITCODE"
            exit $LASTEXITCODE
        }
        
        if (-not $IsDryRun) {
            # Wait for pods to be ready
            if (Wait-ForPods -Namespace $Namespace -ReleaseName $ReleaseName) {
                Show-DataEngineAccess -Namespace $Namespace -ReleaseName $ReleaseName
            } else {
                Write-Warning "Deployment may not be fully ready. Check pod status manually."
                Write-Host "kubectl get pods -n $Namespace -l app.kubernetes.io/instance=$ReleaseName"
            }
        } else {
            Write-Success "Dry run completed successfully!"
        }
        
    } catch {
        Write-Error "Deployment failed: $($_.Exception.Message)"
        exit 1
    }
}

# Main execution
Write-Header "DataEngine Deployment"
Write-Host "Release Name: $ReleaseName"
Write-Host "Namespace: $Namespace"
Write-Host "Values File: $ValuesFile"
Write-Host "Database-Only Mode: $DatabaseOnly"
Write-Host "Mode: $(if ($Upgrade) { 'Upgrade' } else { 'Install' })"

if ($DryRun) {
    Write-Host "DRY RUN MODE - No actual changes will be made"
}

Write-Host ""
if ($DatabaseOnly) {
    Write-Host "This will deploy DataEngine in DATABASE-ONLY mode:"
    Write-Host "- Websocket data ingestion and persistence"
    Write-Host "- Database and cache operations"
    Write-Host "- NO message broker functionality"
} else {
    Write-Host "This will deploy DataEngine in FULL mode:"
    Write-Host "- Complete DataEngine functionality"
    Write-Host "- Message broker operations"
    Write-Host "- Database and cache operations"
}
Write-Host ""

if (-not $DryRun) {
    $confirm = Read-Host "Continue with deployment? (y/N)"
    if ($confirm -ne "y" -and $confirm -ne "Y") {
        Write-Host "Deployment cancelled."
        exit 0
    }
}

Deploy-DataEngine -ReleaseName $ReleaseName -Namespace $Namespace -ValuesFile $ValuesFile -IsUpgrade $Upgrade -IsDryRun $DryRun -DatabaseOnlyMode $DatabaseOnly
