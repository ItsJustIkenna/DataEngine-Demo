# Kubernetes Deployment

## Directory Structure

```
k8s/
└── data-engine-helm/          # Production-ready Helm chart
    ├── Chart.yaml             # Chart metadata
    ├── values.yaml            # Default configuration
    ├── values-dev.yaml        # Development overrides
    ├── values-prod.yaml       # Production overrides
    ├── values-qa.yaml         # QA overrides
    └── templates/             # Kubernetes manifests
        ├── deployment.yaml
        ├── service.yaml
        ├── postgresql-deployment.yaml
        ├── redis-deployment.yaml
        ├── migration-job.yaml
        ├── configmap.yaml
        ├── secret.yaml
        └── ...
```

## Deployment Methods

### 1. Automated CI/CD (Recommended)
The GitHub Actions workflow automatically deploys on push to `main`:
- Builds and pushes Docker image
- Deploys using Helm with environment-specific values
- Runs database migrations
- Performs health checks

### 2. Manual Deployment

#### Development Environment
```powershell
helm upgrade --install dataengine ./k8s/data-engine-helm `
  --namespace dataengine `
  --create-namespace `
  --values ./k8s/data-engine-helm/values-dev.yaml `
  --wait
```

#### Production Environment
```powershell
helm upgrade --install dataengine ./k8s/data-engine-helm `
  --namespace dataengine `
  --create-namespace `
  --values ./k8s/data-engine-helm/values-prod.yaml `
  --set image.tag=v1.2.3 `
  --wait
```

#### Custom Values
```powershell
helm upgrade --install dataengine ./k8s/data-engine-helm `
  --namespace dataengine `
  --create-namespace `
  --set image.tag=latest `
  --set replicaCount=3 `
  --set customPostgresql.enabled=true `
  --set customRedis.enabled=true `
  --wait
```

## Required Secrets

Create these secrets in your cluster before deploying:

```powershell
# Database credentials
kubectl create secret generic database-credentials \
  --from-literal=password=YOUR_POSTGRES_PASSWORD \
  -n dataengine

# Redis credentials
kubectl create secret generic redis-credentials \
  --from-literal=password=YOUR_REDIS_PASSWORD \
  -n dataengine

# GHCR image pull secret
kubectl create secret docker-registry ghcr-secret \
  --docker-server=ghcr.io \
  --docker-username=YOUR_GITHUB_USERNAME \
  --docker-password=YOUR_GHCR_TOKEN \
  -n dataengine
```

## Helm Commands

### Check Status
```powershell
helm status dataengine -n dataengine
```

### View Values
```powershell
helm get values dataengine -n dataengine
```

### Rollback
```powershell
helm rollback dataengine -n dataengine
```

### Uninstall
```powershell
helm uninstall dataengine -n dataengine
```

### Template Validation
```powershell
# Dry-run to see rendered templates
helm template dataengine ./k8s/data-engine-helm \
  --values ./k8s/data-engine-helm/values-dev.yaml

# Validate without installing
helm install dataengine ./k8s/data-engine-helm --dry-run --debug
```

## Components Deployed

| Component | Type | Purpose |
|-----------|------|---------|
| DataEngine | Deployment | Main application (2 replicas) |
| PostgreSQL | StatefulSet | Database with PostGIS extension |
| Redis | StatefulSet | Caching and pub/sub |
| Migrations | Job | Database schema migrations |
| Services | ClusterIP | Internal service discovery |
| ConfigMap | ConfigMap | Application configuration |
| Secrets | Secret | Sensitive credentials |

## Health Checks

The deployment includes:
- **Liveness Probes**: Restart unhealthy pods
- **Readiness Probes**: Control traffic routing
- **Startup Probes**: Allow slow-starting apps time to boot

## Monitoring

Access metrics:
```powershell
# Port-forward to metrics endpoint
kubectl port-forward -n dataengine svc/dataengine 9090:9090

# View metrics
curl http://localhost:9090/metrics
```

## Troubleshooting

### View Logs
```powershell
# Application logs
kubectl logs -n dataengine -l app.kubernetes.io/name=data-engine --tail=100 -f

# Migration logs
kubectl logs -n dataengine -l job-name=dataengine-migration

# PostgreSQL logs
kubectl logs -n dataengine -l app.kubernetes.io/component=postgresql
```

### Debug Pod
```powershell
kubectl exec -it -n dataengine deployment/dataengine -- /bin/sh
```

### Check Resources
```powershell
kubectl get all -n dataengine
kubectl describe pod -n dataengine POD_NAME
```

## Migration from kubectl to Helm

The old `simple/` and `plain-yaml/` directories have been **removed**. All deployments now use Helm for:
- ✅ Templating and reusability
- ✅ Environment-specific configurations
- ✅ Version control and rollbacks
- ✅ Dependency management
- ✅ Single source of truth
