# DataEngine

High-performance cryptocurrency market data processing engine built with Rust and deployed on Kubernetes using Helm.

## 🚀 Quick Start

### Prerequisites
- **Docker Desktop** with Kubernetes enabled
- **kubectl** configured and connected to cluster  
- **Helm 3.x** installed (`winget install Helm.Helm`)

### Deploy DataEngine

```powershell
# Navigate to DataEngine directory
cd DataEngine

# Deploy development environment
.\scripts\deploy.ps1

# Check status
.\scripts\deploy.ps1 -Status

# Access the application (in new terminal)
kubectl port-forward -n data-engine-dev service/data-engine 8080:8080

# Open in browser
Start-Process "http://localhost:8080"
```

## 📋 Available Commands

### Deployment
```powershell
# Development environment
.\scripts\deploy.ps1 -Environment dev

# Production environment  
.\scripts\deploy.ps1 -Environment prod

# With custom secrets
.\scripts\deploy.ps1 -SecretsFile secrets.yaml

# Clean redeployment
.\scripts\deploy.ps1 -Clean
```

### Management
```powershell
# Check deployment status
.\scripts\deploy.ps1 -Status

# View logs
.\scripts\logs.ps1 -Follow

# Health check
.\scripts\health-check.ps1 -Detailed

# Stop deployment
.\scripts\deploy.ps1 -Stop
```

### Using Makefile
```bash
make dev          # Deploy development
make prod         # Deploy production  
make status       # Check status
make logs         # View logs
make stop         # Stop deployment
```

## 🔑 Secret Management

### Development (Quick Start)
```powershell
# Uses default development secrets - no setup needed
.\scripts\deploy.ps1
```

### Production (Secure)
```powershell
# 1. Create secrets file from template
Copy-Item secrets.template.yaml secrets.yaml

# 2. Edit with your actual credentials
notepad secrets.yaml

# 3. Deploy with secrets
.\scripts\deploy.ps1 -Environment prod -SecretsFile secrets.yaml
```

## 🏗️ Architecture

- **Rust Application**: High-performance market data processing
- **PostgreSQL + TimescaleDB**: Time-series database for market data
- **Redis**: High-speed caching and session storage
- **Kubernetes**: Container orchestration and scaling
- **Helm**: Configuration management and deployment
- **Prometheus + Grafana**: Monitoring and observability (optional)

## 📊 Environments

| Environment | Namespace | Resources | Use Case |
|-------------|-----------|-----------|----------|
| **dev** | data-engine-dev | 1 CPU, 1GB RAM | Development, testing |
| **qa** | data-engine-qa | 1 CPU, 2GB RAM | Quality assurance |
| **prod** | data-engine | 2 CPU, 4GB RAM | Production deployment |
| **monitoring** | data-engine-monitoring | + Prometheus/Grafana | Full observability |

## 🔧 Advanced Usage

### Access Services
```powershell
# DataEngine API
kubectl port-forward -n data-engine-dev service/data-engine 8080:8080

# Database (PostgreSQL)
kubectl port-forward -n data-engine-dev service/postgresql 5432:5432

# Cache (Redis)
kubectl port-forward -n data-engine-dev service/redis 6379:6379

# Metrics (Prometheus)
kubectl port-forward -n data-engine-dev service/prometheus 9090:9090

# Dashboards (Grafana)
kubectl port-forward -n data-engine-dev service/grafana 3000:3000
```

### Database Operations
```powershell
# Create backup
.\scripts\database.ps1 -Action backup

# Restore from backup
.\scripts\database.ps1 -Action restore

# Check database status
.\scripts\database.ps1 -Action status
```

### Log Management
```powershell
# Follow application logs
.\scripts\logs.ps1 -Follow

# Export logs to files
.\scripts\logs.ps1 -Export

# Filter for errors
.\scripts\logs.ps1 -Filter "ERROR" -Since "1h"
```

## 🛠️ Development Workflow

```powershell
# 1. Deploy development environment
.\scripts\deploy.ps1 -Environment dev

# 2. Make code changes in your IDE

# 3. Redeploy with latest changes
.\scripts\deploy.ps1 -Clean

# 4. Monitor logs
.\scripts\logs.ps1 -Follow

# 5. Check health
.\scripts\health-check.ps1

# 6. Access application
kubectl port-forward -n data-engine-dev service/data-engine 8080:8080
```

## 📚 Documentation

- **Scripts**: See `scripts/README.md` for detailed script documentation
- **Helm Chart**: See `k8s/data-engine-helm/` for Kubernetes configuration
- **API Docs**: Available at `http://localhost:8080/docs` when running

## 🚨 Important Notes

1. **Never commit secrets**: `secrets.yaml` files should never be committed to version control
2. **Kubernetes required**: This deployment requires a running Kubernetes cluster
3. **Resource requirements**: Ensure adequate CPU/memory for your target environment
4. **Network access**: Cluster must have internet access to pull container images

## 🆘 Troubleshooting

### Common Issues

**Deployment fails**:
```powershell
# Check cluster connectivity
kubectl cluster-info

# Check pod status
kubectl get pods -n data-engine-dev

# Check events
kubectl get events -n data-engine-dev --sort-by='.lastTimestamp'
```

**Cannot access application**:
```powershell
# Verify port-forward is running
kubectl port-forward -n data-engine-dev service/data-engine 8080:8080

# Check service status
kubectl get services -n data-engine-dev
```

**Resource issues**:
```powershell
# Check resource usage
kubectl top pods -n data-engine-dev

# Scale down for development
helm upgrade data-engine ./k8s/data-engine-helm --set replicaCount=1
```

For more help, check the logs:
```powershell
.\scripts\logs.ps1 -Filter "ERROR" -Detailed
```
