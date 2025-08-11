# DataEngine Helm Chart

A comprehensive Helm chart for deploying the DataEngine trading platform on Kubernetes with production-ready configurations.

## Overview

This chart deploys a high-performance Rust-based trading data engine with:
- Real-time data processing capabilities
- PostgreSQL/TimescaleDB for time-series data
- Redis for caching and session management
- Prometheus monitoring and metrics
- Grafana dashboards
- Auto-scaling and high availability
- Security hardening and network policies

## Prerequisites

- Kubernetes cluster 1.20+
- Helm 3.8+
- Available storage class for persistent volumes
- (Optional) Ingress controller for external access
- (Optional) Prometheus operator for monitoring

## Quick Start

### 1. Add Required Helm Repositories

```bash
helm repo add bitnami https://charts.bitnami.com/bitnami
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo update
```

### 2. Install with Default Values (Development)

```bash
helm install data-engine ./data-engine-helm
```

### 3. Install for Specific Environment

```bash
# Development
helm install data-engine ./data-engine-helm -f values-dev.yaml

# QA/Staging
helm install data-engine ./data-engine-helm -f values-qa.yaml

# Production
helm install data-engine ./data-engine-helm -f values-prod.yaml
```

## Configuration

### Core Application Settings

| Parameter | Description | Default |
|-----------|-------------|---------|
| `replicaCount` | Number of DataEngine replicas | `1` |
| `image.repository` | Container image repository | `data-engine` |
| `image.tag` | Container image tag | `latest` |
| `image.pullPolicy` | Image pull policy | `IfNotPresent` |

### Resource Management

| Parameter | Description | Default |
|-----------|-------------|---------|
| `resources.limits.cpu` | CPU limit | `2000m` |
| `resources.limits.memory` | Memory limit | `4Gi` |
| `resources.requests.cpu` | CPU request | `1000m` |
| `resources.requests.memory` | Memory request | `2Gi` |

### Auto-scaling

| Parameter | Description | Default |
|-----------|-------------|---------|
| `autoscaling.enabled` | Enable horizontal pod autoscaling | `true` |
| `autoscaling.minReplicas` | Minimum replicas | `3` |
| `autoscaling.maxReplicas` | Maximum replicas | `10` |
| `autoscaling.targetCPUUtilizationPercentage` | Target CPU utilization | `70` |

### Database Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `postgresql.enabled` | Enable PostgreSQL | `true` |
| `postgresql.auth.database` | Database name | `dataengine` |
| `postgresql.primary.persistence.size` | PostgreSQL storage size | `100Gi` |
| `redis.enabled` | Enable Redis | `true` |
| `redis.auth.enabled` | Enable Redis authentication | `true` |

### Monitoring

| Parameter | Description | Default |
|-----------|-------------|---------|
| `monitoring.enabled` | Enable monitoring | `true` |
| `monitoring.metrics.enabled` | Enable metrics collection | `true` |
| `monitoring.serviceMonitor.enabled` | Create ServiceMonitor for Prometheus | `true` |

### Security

| Parameter | Description | Default |
|-----------|-------------|---------|
| `podSecurityContext.runAsNonRoot` | Run as non-root user | `true` |
| `securityContext.allowPrivilegeEscalation` | Allow privilege escalation | `false` |
| `securityContext.readOnlyRootFilesystem` | Read-only root filesystem | `true` |
| `networkPolicy.enabled` | Enable network policies | `true` |

## Environment-Specific Configurations

### Development (`values-dev.yaml`)
- Single replica for development
- Relaxed security policies
- Debug logging enabled
- No resource limits
- Local storage for databases

### QA/Staging (`values-qa.yaml`)
- 2 replicas for basic availability
- Production-like configuration with reduced resources
- Comprehensive logging
- Persistent storage enabled
- Network policies enforced

### Production (`values-prod.yaml`)
- 3+ replicas for high availability
- Maximum security hardening
- Optimized resource allocation
- High-performance storage
- Full monitoring and alerting
- TLS termination
- Advanced autoscaling

## Deployment Patterns

### Standard Deployment
```bash
helm install data-engine ./data-engine-helm \
  --namespace trading \
  --create-namespace \
  -f values-prod.yaml
```

### Upgrade Deployment
```bash
helm upgrade data-engine ./data-engine-helm \
  --namespace trading \
  -f values-prod.yaml \
  --timeout 10m
```

### Rollback
```bash
helm rollback data-engine 1 --namespace trading
```

## Monitoring and Observability

### Metrics
The DataEngine exposes metrics on port 9090:
- Application performance metrics
- Trading data processing rates
- Connection status to exchanges
- Resource utilization

### Health Checks
- **Liveness Probe**: `/health/live` - Ensures container is running
- **Readiness Probe**: `/health/ready` - Ensures service is ready to accept traffic
- **Startup Probe**: `/health/startup` - Handles slow startup scenarios

### Dashboards
Pre-configured Grafana dashboards are available for:
- Application performance
- Trading data flows
- Infrastructure metrics
- Business KPIs

## Security Features

### Pod Security
- Non-root user execution
- Read-only root filesystem
- No privilege escalation
- Dropped Linux capabilities

### Network Security
- NetworkPolicy restricts traffic
- Service mesh ready
- TLS encryption for external traffic

### Secret Management
- Encrypted secrets for database credentials
- API keys stored securely
- Rotation-ready configuration

## Troubleshooting

### Common Issues

1. **Pod Stuck in Pending**
   ```bash
   kubectl describe pod <pod-name> -n trading
   ```
   Check for resource constraints or storage issues.

2. **Database Connection Issues**
   ```bash
   kubectl logs <pod-name> -n trading
   ```
   Verify database credentials and connectivity.

3. **Performance Issues**
   ```bash
   kubectl top pod -n trading
   kubectl describe hpa data-engine -n trading
   ```
   Check resource usage and scaling behavior.

### Debug Mode
Enable debug mode by setting:
```yaml
config:
  log_level: "debug"
  rust_log: "data_engine=debug"
```

## Scaling Considerations

### Vertical Scaling
Increase resources for single pods:
```yaml
resources:
  limits:
    cpu: "4000m"
    memory: "8Gi"
```

### Horizontal Scaling
Configure autoscaling parameters:
```yaml
autoscaling:
  minReplicas: 5
  maxReplicas: 20
  targetCPUUtilizationPercentage: 60
```

## Backup and Recovery

### Database Backup
PostgreSQL is configured with automated backups:
```bash
kubectl exec -it postgresql-0 -n trading -- pg_dump dataengine > backup.sql
```

### Configuration Backup
```bash
helm get values data-engine -n trading > current-config.yaml
```

## Performance Tuning

### CPU Optimization
- Set CPU affinity for predictable performance
- Configure NUMA topology awareness
- Tune Rust runtime worker threads

### Memory Optimization
- Enable huge pages for large datasets
- Configure memory allocation strategies
- Set appropriate heap sizes

### Network Optimization
- Use SR-IOV for low-latency networking
- Configure kernel bypass networking
- Optimize TCP settings

## Support and Maintenance

### Version Updates
```bash
# Check current version
helm list -n trading

# Update to new version
helm upgrade data-engine ./data-engine-helm \
  --namespace trading \
  --set image.tag=v1.2.0
```

### Health Monitoring
```bash
# Check all components
kubectl get all -n trading

# Monitor logs
kubectl logs -f deployment/data-engine -n trading

# Check metrics
curl http://data-engine-service:9090/metrics
```

## Contributing

1. Test changes in development environment first
2. Update documentation for any new parameters
3. Ensure backward compatibility
4. Add appropriate validation rules

## License

This Helm chart is licensed under the same terms as the DataEngine application.
