# DataEngine Deployment Guide

## Overview

The DataEngine uses **Helm charts as the primary deployment method** for production environments. This approach provides:

- **Professional deployment orchestration**
- **Environment-specific configuration management**
- **Resource management and security**
- **Health checks and monitoring integration**
- **Easy scaling and upgrades**

## Deployment Options

### 1. Production Deployment (Recommended)

#### Full Messaging Mode
```bash
# Deploy complete DataEngine with message broker
helm install dataengine ./k8s/data-engine-helm \
  -f ./k8s/data-engine-helm/values-prod.yaml \
  --namespace trading-platform \
  --create-namespace
```

#### Database-Only Mode
```bash
# Deploy optimized for data collection only
helm install dataengine-collector ./k8s/data-engine-helm \
  -f ./k8s/data-engine-helm/values-database-only.yaml \
  --namespace data-collection \
  --create-namespace
```

### 2. Development Environment

```bash
# Quick development setup
helm install dataengine-dev ./k8s/data-engine-helm \
  -f ./k8s/data-engine-helm/values-dev.yaml \
  --namespace dev
```

### 3. QA Environment

```bash
# QA environment with testing optimizations
helm install dataengine-qa ./k8s/data-engine-helm \
  -f ./k8s/data-engine-helm/values-qa.yaml \
  --namespace qa
```

## Configuration Management

### Available Values Files

| File | Purpose | Use Case |
|------|---------|-----------|
| `values.yaml` | Base configuration | Default settings |
| `values-prod.yaml` | Production ready | High availability, security |
| `values-dev.yaml` | Development | Resource efficient, debug enabled |
| `values-qa.yaml` | QA testing | Testing optimizations |
| `values-database-only.yaml` | Database-only mode | Data collection without messaging |

### Key Configuration Options

#### Database-Only Mode
```yaml
config:
  database_only_mode: true
  disable_message_broker: true
  disable_publisher: true
```

#### Resource Scaling
```yaml
replicaCount: 3
resources:
  requests:
    cpu: "1000m"
    memory: "2Gi"
  limits:
    cpu: "2000m" 
    memory: "4Gi"
```

#### Database Configuration
```yaml
config:
  database:
    max_connections: 100
    connection_timeout: "10s"
    retry_attempts: 3
```

## Deployment Commands Reference

### Install/Upgrade

```bash
# Fresh installation
helm install [RELEASE_NAME] ./k8s/data-engine-helm [OPTIONS]

# Upgrade existing deployment
helm upgrade [RELEASE_NAME] ./k8s/data-engine-helm [OPTIONS]

# Install or upgrade (idempotent)
helm upgrade --install [RELEASE_NAME] ./k8s/data-engine-helm [OPTIONS]
```

### Configuration Override

```bash
# Override single values
helm install dataengine ./k8s/data-engine-helm \
  --set config.database_only_mode=true \
  --set replicaCount=5

# Override multiple values with file
helm install dataengine ./k8s/data-engine-helm \
  -f custom-values.yaml

# Combine base and override files
helm install dataengine ./k8s/data-engine-helm \
  -f ./k8s/data-engine-helm/values-prod.yaml \
  -f custom-overrides.yaml
```

### Environment Management

```bash
# Production namespace
helm install dataengine ./k8s/data-engine-helm \
  -f values-prod.yaml \
  --namespace trading-prod \
  --create-namespace

# Staging environment
helm install dataengine-staging ./k8s/data-engine-helm \
  -f values-qa.yaml \
  --namespace trading-staging \
  --create-namespace
```

## Monitoring and Management

### Check Deployment Status

```bash
# List all releases
helm list --all-namespaces

# Check specific release
helm status dataengine -n trading-platform

# Get deployment info
kubectl get pods,svc,ing -n trading-platform

# View logs
kubectl logs -f deployment/dataengine -n trading-platform
```

### Health Checks

```bash
# Check application health
kubectl port-forward svc/dataengine 8080:8080 -n trading-platform
curl http://localhost:8080/health

# View metrics
curl http://localhost:8080/metrics
```

### Scaling

```bash
# Scale replicas
helm upgrade dataengine ./k8s/data-engine-helm \
  --set replicaCount=5 \
  --reuse-values

# Update resources
helm upgrade dataengine ./k8s/data-engine-helm \
  --set resources.requests.cpu=2000m \
  --set resources.requests.memory=4Gi \
  --reuse-values
```

## Troubleshooting

### Common Issues

#### Pod Startup Failures
```bash
# Check pod status
kubectl describe pod [POD_NAME] -n [NAMESPACE]

# View container logs
kubectl logs [POD_NAME] -c data-engine -n [NAMESPACE]

# Check events
kubectl get events -n [NAMESPACE] --sort-by='.lastTimestamp'
```

#### Database Connection Issues
```bash
# Check database connectivity
kubectl exec -it [POD_NAME] -n [NAMESPACE] -- nc -zv postgres 5432

# Verify environment variables
kubectl exec -it [POD_NAME] -n [NAMESPACE] -- env | grep -i database
```

#### Configuration Issues
```bash
# Check current values
helm get values dataengine -n trading-platform

# Validate Helm chart
helm lint ./k8s/data-engine-helm

# Dry run deployment
helm install dataengine ./k8s/data-engine-helm \
  --dry-run --debug
```

## Best Practices

### 1. Use Namespace Isolation

```bash
# Separate namespaces for different environments
kubectl create namespace trading-prod
kubectl create namespace trading-staging
kubectl create namespace data-collection
```

### 2. Resource Management

```yaml
# Always set resource requests and limits
resources:
  requests:
    cpu: "1000m"
    memory: "2Gi"
  limits:
    cpu: "2000m"
    memory: "4Gi"
```

### 3. Security Configuration

```yaml
# Use security contexts
podSecurityContext:
  runAsNonRoot: true
  runAsUser: 1000
  fsGroup: 1000

securityContext:
  allowPrivilegeEscalation: false
  readOnlyRootFilesystem: true
  capabilities:
    drop: ["ALL"]
```

### 4. Environment-Specific Values

```bash
# Use environment-specific value files
production: values-prod.yaml
staging: values-qa.yaml
development: values-dev.yaml
data-collection: values-database-only.yaml
```

### 5. Version Management

```bash
# Always specify chart version in production
helm install dataengine ./k8s/data-engine-helm \
  --version 1.0.0 \
  -f values-prod.yaml
```

## Migration from Scripts

If you previously used deployment scripts:

1. **Environment Variables** → Move to Helm values files
2. **Manual Configuration** → Use Helm templates
3. **Resource Management** → Leverage Kubernetes resource management
4. **Health Checks** → Use Kubernetes probes
5. **Scaling** → Use Helm upgrade commands

### Example Migration

**Old Script Approach:**
```bash
export DATABASE_ONLY_MODE=true
export POSTGRES_HOST=localhost
docker run -e DATABASE_ONLY_MODE=true dataengine:latest
```

**New Helm Approach:**
```bash
helm install dataengine ./k8s/data-engine-helm \
  --set config.database_only_mode=true \
  --set postgresql.enabled=true
```

## Support

- **Helm Chart Issues**: Check `helm status` and `kubectl describe`
- **Application Issues**: Review application logs via `kubectl logs`
- **Resource Issues**: Monitor with `kubectl top pods` and metrics endpoints
- **Configuration Issues**: Validate with `helm lint` and dry-run deployments
