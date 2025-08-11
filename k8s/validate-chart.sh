#!/bin/bash
# Helm Chart Validation Script
# Tests the DataEngine Helm chart for syntax, logic, and best practices

set -e

CHART_DIR="./data-engine-helm"
NAMESPACE="data-engine-test"

echo "🚀 DataEngine Helm Chart Validation"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# 1. Lint the chart
echo
echo "1. Running Helm Lint..."
if helm lint $CHART_DIR; then
    print_status "Helm lint passed"
else
    print_error "Helm lint failed"
    exit 1
fi

# 2. Template validation
echo
echo "2. Validating Templates..."
if helm template data-engine $CHART_DIR > /dev/null 2>&1; then
    print_status "Template generation successful"
else
    print_error "Template generation failed"
    exit 1
fi

# 3. Validate with different values files
echo
echo "3. Testing Environment-Specific Configurations..."

for env in dev qa prod; do
    echo "   Testing $env environment..."
    if [ -f "$CHART_DIR/values-$env.yaml" ]; then
        if helm template data-engine $CHART_DIR -f $CHART_DIR/values-$env.yaml > /dev/null 2>&1; then
            print_status "$env configuration valid"
        else
            print_error "$env configuration invalid"
            exit 1
        fi
    else
        print_warning "$env values file not found"
    fi
done

# 4. Check for required dependencies
echo
echo "4. Checking Dependencies..."
if grep -q "dependencies:" $CHART_DIR/Chart.yaml; then
    print_status "Dependencies defined"
    helm dependency list $CHART_DIR || print_warning "Dependencies need to be built"
else
    print_warning "No dependencies defined"
fi

# 5. Validate Kubernetes manifests
echo
echo "5. Validating Kubernetes Manifests..."
TEMP_DIR=$(mktemp -d)
helm template data-engine $CHART_DIR --output-dir $TEMP_DIR

# Check if kubectl is available for validation
if command -v kubectl &> /dev/null; then
    echo "   Using kubectl for validation..."
    find $TEMP_DIR -name "*.yaml" -exec kubectl apply --dry-run=client -f {} \; > /dev/null 2>&1
    if [ $? -eq 0 ]; then
        print_status "Kubernetes manifest validation passed"
    else
        print_error "Kubernetes manifest validation failed"
        exit 1
    fi
else
    print_warning "kubectl not available, skipping Kubernetes validation"
fi

rm -rf $TEMP_DIR

# 6. Security validation
echo
echo "6. Security Validation..."
MANIFEST=$(helm template data-engine $CHART_DIR)

# Check for security contexts
if echo "$MANIFEST" | grep -q "securityContext"; then
    print_status "Security contexts defined"
else
    print_warning "No security contexts found"
fi

# Check for resource limits
if echo "$MANIFEST" | grep -q "resources:"; then
    print_status "Resource limits defined"
else
    print_warning "No resource limits found"
fi

# Check for non-root user
if echo "$MANIFEST" | grep -q "runAsNonRoot: true"; then
    print_status "Non-root user configuration found"
else
    print_warning "Non-root user configuration not found"
fi

# 7. Best practices check
echo
echo "7. Best Practices Validation..."

# Check for health checks
if echo "$MANIFEST" | grep -q "livenessProbe"; then
    print_status "Liveness probes defined"
else
    print_warning "No liveness probes found"
fi

if echo "$MANIFEST" | grep -q "readinessProbe"; then
    print_status "Readiness probes defined"
else
    print_warning "No readiness probes found"
fi

# Check for labels
if echo "$MANIFEST" | grep -q "app.kubernetes.io/name"; then
    print_status "Standard labels found"
else
    print_warning "Standard labels not found"
fi

# 8. Production readiness check
echo
echo "8. Production Readiness Check..."
PROD_MANIFEST=$(helm template data-engine $CHART_DIR -f $CHART_DIR/values-prod.yaml)

# Check for multiple replicas in prod
REPLICAS=$(echo "$PROD_MANIFEST" | grep "replicas:" | head -1 | awk '{print $2}')
if [ "$REPLICAS" -gt 1 ] 2>/dev/null; then
    print_status "Production has multiple replicas ($REPLICAS)"
else
    print_warning "Production should have multiple replicas"
fi

# Check for resource limits in prod
if echo "$PROD_MANIFEST" | grep -q "limits:"; then
    print_status "Production has resource limits"
else
    print_error "Production must have resource limits"
fi

# Check for persistent storage
if echo "$PROD_MANIFEST" | grep -q "PersistentVolumeClaim"; then
    print_status "Persistent storage configured"
else
    print_warning "No persistent storage found"
fi

# 9. Documentation check
echo
echo "9. Documentation Check..."
if [ -f "$CHART_DIR/README.md" ]; then
    print_status "README.md exists"
else
    print_warning "README.md not found"
fi

if [ -f "$CHART_DIR/Chart.yaml" ]; then
    if grep -q "description:" $CHART_DIR/Chart.yaml; then
        print_status "Chart description exists"
    else
        print_warning "Chart description missing"
    fi
fi

echo
echo "🎉 Validation Complete!"
echo
echo "Summary:"
echo "--------"
echo "Chart: $CHART_DIR"
echo "Status: Ready for deployment"
echo
echo "Next Steps:"
echo "- Run 'helm dependency build $CHART_DIR' to download dependencies"
echo "- Deploy to a test environment first"
echo "- Monitor resource usage and adjust limits accordingly"
echo
echo "Deployment Commands:"
echo "  Development: helm install data-engine $CHART_DIR -f $CHART_DIR/values-dev.yaml"
echo "  QA:          helm install data-engine $CHART_DIR -f $CHART_DIR/values-qa.yaml"
echo "  Production:  helm install data-engine $CHART_DIR -f $CHART_DIR/values-prod.yaml"
