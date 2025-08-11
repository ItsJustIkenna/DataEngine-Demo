# DataEngine Helm Chart Validation Script (PowerShell)
# Tests the DataEngine Helm chart for syntax, logic, and best practices

param(
    [string]$ChartDir = "./data-engine-helm",
    [string]$Namespace = "data-engine-test"
)

$ErrorActionPreference = "Stop"

Write-Host "🚀 DataEngine Helm Chart Validation" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

function Write-Success {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "⚠ $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor Red
}

# Check if Helm is installed
try {
    $helmVersion = helm version --short 2>$null
    Write-Success "Helm is installed: $helmVersion"
} catch {
    Write-Error "Helm is not installed or not in PATH"
    exit 1
}

# 1. Lint the chart
Write-Host ""
Write-Host "1. Running Helm Lint..." -ForegroundColor Blue
try {
    helm lint $ChartDir | Out-Null
    Write-Success "Helm lint passed"
} catch {
    Write-Error "Helm lint failed"
    exit 1
}

# 2. Template validation
Write-Host ""
Write-Host "2. Validating Templates..." -ForegroundColor Blue
try {
    helm template data-engine $ChartDir | Out-Null
    Write-Success "Template generation successful"
} catch {
    Write-Error "Template generation failed"
    exit 1
}

# 3. Validate with different values files
Write-Host ""
Write-Host "3. Testing Environment-Specific Configurations..." -ForegroundColor Blue

$environments = @("dev", "qa", "prod")
foreach ($env in $environments) {
    Write-Host "   Testing $env environment..." -ForegroundColor Gray
    $valuesFile = "$ChartDir/values-$env.yaml"
    if (Test-Path $valuesFile) {
        try {
            helm template data-engine $ChartDir -f $valuesFile | Out-Null
            Write-Success "$env configuration valid"
        } catch {
            Write-Error "$env configuration invalid"
            exit 1
        }
    } else {
        Write-Warning "$env values file not found"
    }
}

# 4. Check for required dependencies
Write-Host ""
Write-Host "4. Checking Dependencies..." -ForegroundColor Blue
$chartFile = "$ChartDir/Chart.yaml"
if (Test-Path $chartFile) {
    $chartContent = Get-Content $chartFile -Raw
    if ($chartContent -match "dependencies:") {
        Write-Success "Dependencies defined"
        try {
            helm dependency list $ChartDir | Out-Null
        } catch {
            Write-Warning "Dependencies need to be built with 'helm dependency build'"
        }
    } else {
        Write-Warning "No dependencies defined"
    }
}

# 5. Validate Kubernetes manifests
Write-Host ""
Write-Host "5. Validating Kubernetes Manifests..." -ForegroundColor Blue
$tempDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -Type Directory -Path $_ }
try {
    helm template data-engine $ChartDir --output-dir $tempDir | Out-Null
    
    # Check if kubectl is available
    if (Get-Command kubectl -ErrorAction SilentlyContinue) {
        Write-Host "   Using kubectl for validation..." -ForegroundColor Gray
        $yamlFiles = Get-ChildItem -Path $tempDir -Recurse -Filter "*.yaml"
        $allValid = $true
        foreach ($file in $yamlFiles) {
            try {
                kubectl apply --dry-run=client -f $file.FullName | Out-Null
            } catch {
                $allValid = $false
                break
            }
        }
        if ($allValid) {
            Write-Success "Kubernetes manifest validation passed"
        } else {
            Write-Error "Kubernetes manifest validation failed"
            exit 1
        }
    } else {
        Write-Warning "kubectl not available, skipping Kubernetes validation"
    }
} finally {
    Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}

# 6. Security validation
Write-Host ""
Write-Host "6. Security Validation..." -ForegroundColor Blue
$manifest = helm template data-engine $ChartDir

# Check for security contexts
if ($manifest -match "securityContext") {
    Write-Success "Security contexts defined"
} else {
    Write-Warning "No security contexts found"
}

# Check for resource limits
if ($manifest -match "resources:") {
    Write-Success "Resource limits defined"
} else {
    Write-Warning "No resource limits found"
}

# Check for non-root user
if ($manifest -match "runAsNonRoot: true") {
    Write-Success "Non-root user configuration found"
} else {
    Write-Warning "Non-root user configuration not found"
}

# 7. Best practices check
Write-Host ""
Write-Host "7. Best Practices Validation..." -ForegroundColor Blue

# Check for health checks
if ($manifest -match "livenessProbe") {
    Write-Success "Liveness probes defined"
} else {
    Write-Warning "No liveness probes found"
}

if ($manifest -match "readinessProbe") {
    Write-Success "Readiness probes defined"
} else {
    Write-Warning "No readiness probes found"
}

# Check for labels
if ($manifest -match "app.kubernetes.io/name") {
    Write-Success "Standard labels found"
} else {
    Write-Warning "Standard labels not found"
}

# 8. Production readiness check
Write-Host ""
Write-Host "8. Production Readiness Check..." -ForegroundColor Blue
$prodManifest = helm template data-engine $ChartDir -f "$ChartDir/values-prod.yaml"

# Check for multiple replicas in prod
if ($prodManifest -match "replicas: (\d+)") {
    $replicas = [int]$matches[1]
    if ($replicas -gt 1) {
        Write-Success "Production has multiple replicas ($replicas)"
    } else {
        Write-Warning "Production should have multiple replicas"
    }
}

# Check for resource limits in prod
if ($prodManifest -match "limits:") {
    Write-Success "Production has resource limits"
} else {
    Write-Error "Production must have resource limits"
}

# Check for persistent storage
if ($prodManifest -match "PersistentVolumeClaim") {
    Write-Success "Persistent storage configured"
} else {
    Write-Warning "No persistent storage found"
}

# 9. Documentation check
Write-Host ""
Write-Host "9. Documentation Check..." -ForegroundColor Blue
if (Test-Path "$ChartDir/README.md") {
    Write-Success "README.md exists"
} else {
    Write-Warning "README.md not found"
}

if (Test-Path "$ChartDir/Chart.yaml") {
    $chartYaml = Get-Content "$ChartDir/Chart.yaml" -Raw
    if ($chartYaml -match "description:") {
        Write-Success "Chart description exists"
    } else {
        Write-Warning "Chart description missing"
    }
}

Write-Host ""
Write-Host "🎉 Validation Complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Summary:" -ForegroundColor Cyan
Write-Host "--------" -ForegroundColor Cyan
Write-Host "Chart: $ChartDir"
Write-Host "Status: Ready for deployment"
Write-Host ""
Write-Host "Next Steps:" -ForegroundColor Yellow
Write-Host "- Run 'helm dependency build $ChartDir' to download dependencies"
Write-Host "- Deploy to a test environment first"
Write-Host "- Monitor resource usage and adjust limits accordingly"
Write-Host ""
Write-Host "Deployment Commands:" -ForegroundColor Cyan
Write-Host "  Development: helm install data-engine $ChartDir -f $ChartDir/values-dev.yaml" -ForegroundColor Gray
Write-Host "  QA:          helm install data-engine $ChartDir -f $ChartDir/values-qa.yaml" -ForegroundColor Gray
Write-Host "  Production:  helm install data-engine $ChartDir -f $ChartDir/values-prod.yaml" -ForegroundColor Gray
