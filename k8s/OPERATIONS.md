# DataEngine Deployment Operations Guide

This guide covers operational procedures for deploying and managing the DataEngine trading platform on Kubernetes using Helm.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Initial Setup](#initial-setup)
3. [Environment Deployments](#environment-deployments)
4. [Operational Procedures](#operational-procedures)
5. [Monitoring and Alerts](#monitoring-and-alerts)
6. [Troubleshooting](#troubleshooting)
7. [Disaster Recovery](#disaster-recovery)

## Prerequisites

### Required Tools
- **Kubernetes cluster** (v1.20+)
- **Helm** (v3.8+)
- **kubectl** configured for your cluster
- **Docker** for building images (optional)

### Required Permissions
- Cluster admin access for initial setup
- Namespace admin access for deployments
- Storage provisioner access for PVCs

### Cluster Requirements
- **CPU**: Minimum 4 cores available
- **Memory**: Minimum 8GB available
- **Storage**: Dynamic provisioning enabled
- **Ingress**: Controller installed (optional)
- **Monitoring**: Prometheus operator (optional)

## Initial Setup

### 1. Prepare Helm Repositories

```bash
# Add required repositories
helm repo add bitnami https://charts.bitnami.com/bitnami
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo add ingress-nginx https://kubernetes.github.io/ingress-nginx
helm repo update
```

### 2. Create Namespaces

```bash
# Create environments
kubectl create namespace data-engine-dev
kubectl create namespace data-engine-qa
kubectl create namespace data-engine-prod

# Label namespaces
kubectl label namespace data-engine-dev environment=development
kubectl label namespace data-engine-qa environment=qa
kubectl label namespace data-engine-prod environment=production
```

### 3. Setup Storage Classes (if needed)

```bash
# Example SSD storage class for production
cat <<EOF | kubectl apply -f -
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: fast-ssd
provisioner: kubernetes.io/gce-pd
parameters:
  type: pd-ssd
  replication-type: regional-pd
allowVolumeExpansion: true
EOF
```

### 4. Build Chart Dependencies

```bash
cd k8s/data-engine-helm
helm dependency build
```

## Environment Deployments

### Development Environment

**Purpose**: Local development and feature testing

```bash
# Deploy to development
helm install data-engine-dev ./data-engine-helm \
  --namespace data-engine-dev \
  -f values-dev.yaml \
  --timeout 15m \
  --wait

# Verify deployment
kubectl get all -n data-engine-dev
kubectl logs -f deployment/data-engine-dev -n data-engine-dev
```

**Development Configuration**:
- Single replica
- Minimal resources
- Debug logging
- No persistence (uses emptyDir)
- Relaxed security

### QA/Staging Environment

**Purpose**: Integration testing and pre-production validation

```bash
# Deploy to QA
helm install data-engine-qa ./data-engine-helm \
  --namespace data-engine-qa \
  -f values-qa.yaml \
  --timeout 15m \
  --wait

# Run health checks
kubectl exec deployment/data-engine-qa -n data-engine-qa -- /app/healthcheck
```

**QA Configuration**:
- 2 replicas for availability testing
- Production-like resources (reduced)
- Full persistence enabled
- Security policies enforced
- Monitoring enabled

### Production Environment

**Purpose**: Live trading operations

```bash
# Pre-deployment validation
./validate-chart.ps1 -ChartDir ./data-engine-helm

# Deploy to production
helm install data-engine-prod ./data-engine-helm \
  --namespace data-engine-prod \
  -f values-prod.yaml \
  --timeout 20m \
  --wait \
  --atomic

# Verify critical services
kubectl get pods -n data-engine-prod -o wide
kubectl get pvc -n data-engine-prod
kubectl get svc -n data-engine-prod
```

**Production Configuration**:
- 3+ replicas for high availability
- Maximum resource allocation
- High-performance storage
- Full security hardening
- Complete monitoring stack
- Automated backups

## Operational Procedures

### Rolling Updates

```bash
# Update application version
helm upgrade data-engine-prod ./data-engine-helm \
  --namespace data-engine-prod \
  -f values-prod.yaml \
  --set image.tag=v1.2.0 \
  --timeout 20m \
  --wait

# Monitor rollout
kubectl rollout status deployment/data-engine-prod -n data-engine-prod

# Rollback if needed
helm rollback data-engine-prod 1 --namespace data-engine-prod
```

### Configuration Updates

```bash
# Update configuration only
helm upgrade data-engine-prod ./data-engine-helm \
  --namespace data-engine-prod \
  -f values-prod.yaml \
  --set config.log_level=info \
  --reuse-values

# Restart pods to pick up config changes
kubectl rollout restart deployment/data-engine-prod -n data-engine-prod
```

### Scaling Operations

```bash
# Manual scaling
kubectl scale deployment data-engine-prod --replicas=5 -n data-engine-prod

# Update HPA settings
helm upgrade data-engine-prod ./data-engine-helm \
  --namespace data-engine-prod \
  -f values-prod.yaml \
  --set autoscaling.maxReplicas=15 \
  --reuse-values
```

### Certificate Management

```bash
# Update TLS certificates
kubectl create secret tls data-engine-tls \
  --cert=path/to/cert.pem \
  --key=path/to/key.pem \
  -n data-engine-prod \
  --dry-run=client -o yaml | kubectl apply -f -

# Restart ingress controller if needed
kubectl rollout restart deployment ingress-nginx-controller -n ingress-nginx
```

## Monitoring and Alerts

### Health Checks

```bash
# Check all components
kubectl get all -n data-engine-prod

# Check pod health
kubectl describe pod <pod-name> -n data-engine-prod

# Check application health
kubectl exec deployment/data-engine-prod -n data-engine-prod -- curl -f http://localhost:8080/health

# Check metrics endpoint
kubectl port-forward svc/data-engine-prod 9090:9090 -n data-engine-prod
curl http://localhost:9090/metrics
```

### Performance Monitoring

```bash
# Resource usage
kubectl top pods -n data-engine-prod
kubectl top nodes

# HPA status
kubectl get hpa -n data-engine-prod
kubectl describe hpa data-engine-prod -n data-engine-prod

# PVC usage
kubectl exec deployment/data-engine-prod -n data-engine-prod -- df -h
```

### Log Management

```bash
# Application logs
kubectl logs -f deployment/data-engine-prod -n data-engine-prod

# Previous container logs (in case of crashes)
kubectl logs deployment/data-engine-prod -n data-engine-prod --previous

# All pods logs
kubectl logs -l app.kubernetes.io/name=data-engine -n data-engine-prod

# Export logs
kubectl logs deployment/data-engine-prod -n data-engine-prod --since=1h > app-logs.txt
```

## Troubleshooting

### Common Issues and Solutions

#### 1. Pod Stuck in Pending State

**Symptoms**: Pods remain in Pending state
**Diagnosis**:
```bash
kubectl describe pod <pod-name> -n data-engine-prod
kubectl get events --sort-by=.metadata.creationTimestamp -n data-engine-prod
```

**Common Causes**:
- Insufficient resources
- Storage provisioning issues
- Node selector constraints
- Taints/tolerations mismatch

#### 2. Database Connection Issues

**Symptoms**: Application logs show database connection errors
**Diagnosis**:
```bash
# Check database pod status
kubectl get pods -l app.kubernetes.io/name=postgresql -n data-engine-prod

# Test database connectivity
kubectl exec deployment/data-engine-prod -n data-engine-prod -- nc -zv postgresql 5432

# Check database logs
kubectl logs -l app.kubernetes.io/name=postgresql -n data-engine-prod
```

#### 3. Performance Issues

**Symptoms**: High latency, CPU/memory issues
**Diagnosis**:
```bash
# Resource usage
kubectl top pods -n data-engine-prod
kubectl describe hpa -n data-engine-prod

# Application metrics
kubectl port-forward svc/data-engine-prod 9090:9090 -n data-engine-prod &
curl -s http://localhost:9090/metrics | grep trading_
```

#### 4. Networking Issues

**Symptoms**: Services unreachable, connection timeouts
**Diagnosis**:
```bash
# Check service endpoints
kubectl get endpoints -n data-engine-prod
kubectl describe svc data-engine-prod -n data-engine-prod

# Test internal networking
kubectl exec deployment/data-engine-prod -n data-engine-prod -- nslookup data-engine-prod
kubectl exec deployment/data-engine-prod -n data-engine-prod -- curl -v http://data-engine-prod:8080/health
```

### Emergency Procedures

#### Complete Service Restart

```bash
# Graceful restart
kubectl rollout restart deployment/data-engine-prod -n data-engine-prod
kubectl rollout status deployment/data-engine-prod -n data-engine-prod

# Force restart (last resort)
kubectl delete pods -l app.kubernetes.io/name=data-engine -n data-engine-prod
```

#### Data Recovery

```bash
# Access database directly
kubectl exec -it <postgresql-pod> -n data-engine-prod -- psql -U dataengine

# Restore from backup
kubectl exec -i <postgresql-pod> -n data-engine-prod -- psql -U dataengine < backup.sql
```

## Disaster Recovery

### Backup Procedures

#### Application State Backup

```bash
# Backup Helm configuration
helm get values data-engine-prod -n data-engine-prod > backup/values-$(date +%Y%m%d).yaml
helm get manifest data-engine-prod -n data-engine-prod > backup/manifest-$(date +%Y%m%d).yaml

# Backup secrets
kubectl get secrets -n data-engine-prod -o yaml > backup/secrets-$(date +%Y%m%d).yaml
```

#### Database Backup

```bash
# PostgreSQL backup
kubectl exec <postgresql-pod> -n data-engine-prod -- pg_dump -U dataengine dataengine > backup/db-$(date +%Y%m%d).sql

# Redis backup (if persistent)
kubectl exec <redis-pod> -n data-engine-prod -- redis-cli BGSAVE
kubectl cp <redis-pod>:/data/dump.rdb backup/redis-$(date +%Y%m%d).rdb -n data-engine-prod
```

### Recovery Procedures

#### Complete Environment Recovery

```bash
# 1. Recreate namespace
kubectl delete namespace data-engine-prod
kubectl create namespace data-engine-prod
kubectl label namespace data-engine-prod environment=production

# 2. Restore secrets
kubectl apply -f backup/secrets-latest.yaml

# 3. Deploy fresh instance
helm install data-engine-prod ./data-engine-helm \
  --namespace data-engine-prod \
  -f backup/values-latest.yaml \
  --timeout 20m \
  --wait

# 4. Restore data
kubectl exec -i <postgresql-pod> -n data-engine-prod -- psql -U dataengine < backup/db-latest.sql
```

## Security Operations

### Regular Security Tasks

```bash
# Update container images
helm upgrade data-engine-prod ./data-engine-helm \
  --namespace data-engine-prod \
  -f values-prod.yaml \
  --set image.tag=latest-security-patch

# Rotate secrets
kubectl delete secret data-engine-secrets -n data-engine-prod
kubectl create secret generic data-engine-secrets \
  --from-literal=db-password="$(openssl rand -base64 32)" \
  -n data-engine-prod

# Update TLS certificates
kubectl apply -f updated-tls-secret.yaml
```

### Security Audit

```bash
# Check pod security contexts
kubectl get pods -n data-engine-prod -o jsonpath='{.items[*].spec.securityContext}'

# Verify network policies
kubectl get networkpolicies -n data-engine-prod
kubectl describe networkpolicy data-engine-network-policy -n data-engine-prod

# Check RBAC
kubectl auth can-i --list --as=system:serviceaccount:data-engine-prod:data-engine -n data-engine-prod
```

This operational guide provides comprehensive procedures for managing the DataEngine in production. Regular review and practice of these procedures ensures smooth operations and quick incident response.
