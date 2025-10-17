# DataEngine# DataEngine# DataEngine# DataEngine



High-performance cryptocurrency market data processing engine built with Rust.



---High-performance cryptocurrency market data processing engine built with Rust.



## Prerequisites



- Azure subscription---High-performance cryptocurrency market data processing engine built with Rust.High-performance cryptocurrency market data processing engine built with Rust and deployed on Kubernetes using Helm.

- Azure CLI installed locally

- Kubernetes cluster (docker-desktop)

- Helm 3.x

- Self-hosted GitHub Actions runner (Windows)## ⚠️ Production vs Development Setup



---



## Setup Guide### Current Setup: Development (File-Based Secrets)## Setup (One-Time)## 🚀 Quick Start



### Step 1: Set Up Azure Key Vault- ✅ Quick to set up



Follow the complete guide: **`k8s/AZURE_KEYVAULT_SETUP.md`**- ⚠️ **NOT production-safe** - Plaintext secrets on disk, no audit trail, no encryption



**Quick summary:**

1. Install Azure CLI on your local machine

2. Create Azure Key Vault resource### Recommended: Production (Azure Key Vault)### 1. Create Secrets on Remote Runner### Prerequisites

3. Add all secrets to Key Vault

4. Create service principal for GitHub Actions- ✅ Encrypted secrets at rest

5. Configure GitHub Secrets

- ✅ Full audit trail of all access- **Docker Desktop** with Kubernetes enabled

**Time:** ~30 minutes  

**Cost:** ~$1/month- ✅ RBAC access control



### Step 2: Configure GitHub Secrets- ✅ Automatic rotation support**RDP to your remote Windows runner** and create:- **kubectl** configured and connected to cluster  



Add these secrets to your GitHub repository (Settings → Secrets and variables → Actions):- ✅ Compliance-ready (SOC 2, ISO 27001)



| Secret Name | Description | Example |- 💰 Cost: ~$1/month- **Helm 3.x** installed (`winget install Helm.Helm`)

|-------------|-------------|---------|

| `AZURE_CREDENTIALS` | Service principal JSON | `{"clientId": "...", "clientSecret": "..."}` |

| `AZURE_KEYVAULT_NAME` | Your Key Vault name | `dataengine-kv-prod` |

**To migrate to production secrets:** See `k8s/AZURE_KEYVAULT_SETUP.md````powershell

### Step 3: Deploy



From your local machine:

---# Create directory### Deploy DataEngine

```bash

git push origin main

```

## Quick Start (Development)New-Item -Path "C:\secrets\dataengine" -ItemType Directory -Force

The GitHub Actions workflow will:

1. Authenticate to Azure

2. Fetch secrets from Azure Key Vault

3. Build Docker image### 1. Create Secrets on Remote Runner```powershell

4. Deploy to Kubernetes with Helm

5. Run database migrations



---**RDP to your remote Windows runner** and run:# Create and edit secrets file# Navigate to DataEngine directory



## Verify Deployment



```powershell```powershellnotepad C:\secrets\dataengine\secrets.envcd DataEngine

# Check pods

kubectl get pods -n dataengine# Create directory



# View logsNew-Item -Path "C:\secrets\dataengine" -ItemType Directory -Force```

kubectl logs -n dataengine -l app.kubernetes.io/name=data-engine



# Check health endpoint

kubectl port-forward -n dataengine svc/dataengine-data-engine 8080:8080# Create secrets file# Deploy development environment

# Then visit http://localhost:8080/health

```notepad C:\secrets\dataengine\secrets.env



---```Paste this template with your values:.\scripts\deploy.ps1



## Manage Secrets



### View Secrets (Names Only)**Paste this template with your actual values:**



```powershell

az keyvault secret list --vault-name dataengine-kv-prod -o table

``````ini```ini# Check status



### Update a SecretPOSTGRES_HOST=your-postgres-host.com



```powershellPOSTGRES_PORT=5432POSTGRES_HOST=your-postgres-host.com.\scripts\deploy.ps1 -Status

# Update secret in Key Vault

az keyvault secret set --vault-name dataengine-kv-prod --name "POSTGRES-PASSWORD" --value "new-password"POSTGRES_USER=dataengine



# Trigger redeploymentPOSTGRES_PASSWORD=your-secure-passwordPOSTGRES_PORT=5432

git commit --allow-empty -m "Rotate secrets"

git pushPOSTGRES_DB=dataengine

```

POSTGRES_USER=dataengine# Access the application (in new terminal)

### Rotate All Secrets (Recommended: Every 90 Days)

REDIS_HOST=redis-14099.c282.east-us-mz.azure.redns.redis-cloud.com

```powershell

# Update each secretREDIS_PORT=14099POSTGRES_PASSWORD=your-secure-passwordkubectl port-forward -n data-engine-dev service/data-engine 8080:8080

az keyvault secret set --vault-name dataengine-kv-prod --name "POSTGRES-PASSWORD" --value "new-password"

az keyvault secret set --vault-name dataengine-kv-prod --name "REDIS-PASSWORD" --value "new-password"REDIS_PASSWORD=your-redis-password

az keyvault secret set --vault-name dataengine-kv-prod --name "KRAKEN-API-KEY" --value "new-key"

az keyvault secret set --vault-name dataengine-kv-prod --name "KRAKEN-SECRET-KEY" --value "new-secret"POSTGRES_DB=dataengine



# RedeployKRAKEN_API_KEY=your-kraken-api-key

git commit --allow-empty -m "Rotate all secrets"

git pushKRAKEN_SECRET_KEY=your-kraken-secret-key# Open in browser

```

```

---

REDIS_HOST=redis-14099.c282.east-us-mz.azure.redns.redis-cloud.comStart-Process "http://localhost:8080"

## Architecture

Save and close.

```

┌─────────────────────────────────────────────────────────┐REDIS_PORT=14099```

│              Local Machine (git push)                   │

└────────────────────────┬────────────────────────────────┘**Optional hardening** (still not production-grade):

                         │

                         ▼```powershellREDIS_PASSWORD=your-redis-password

┌─────────────────────────────────────────────────────────┐

│         GitHub Actions (Self-Hosted Runner)             │# Encrypt with Windows EFS

│  ┌──────────────────────────────────────────────────┐   │

│  │  1. Authenticate to Azure                        │   │cipher /e C:\secrets\dataengine\secrets.env## 📋 Available Commands

│  │  2. Fetch secrets from Azure Key Vault           │   │

│  │  3. Build Docker image                           │   │

│  │  4. Deploy to Kubernetes via Helm                │   │

│  └──────────────────────────────────────────────────┘   │# Restrict permissionsKRAKEN_API_KEY=your-kraken-api-key

└────────────────────────┬────────────────────────────────┘

                         │icacls C:\secrets\dataengine\secrets.env /inheritance:r /grant:r "NT AUTHORITY\SYSTEM:(F)"

          ┌──────────────┴──────────────┐

          ▼                             ▼```KRAKEN_SECRET_KEY=your-kraken-secret-key### Deployment

┌─────────────────────┐    ┌─────────────────────────────┐

│  Azure Key Vault    │    │  Kubernetes (docker-desktop)│

│                     │    │  ┌────────────────────────┐ │

│  - POSTGRES_*       │    │  │  DataEngine (Rust)     │ │### 2. Deploy``````powershell

│  - REDIS_*          │    │  │         ↓              │ │

│  - KRAKEN_*         │    │  │  PostgreSQL (External) │ │

│                     │    │  │  Redis (Azure)         │ │

└─────────────────────┘    │  └────────────────────────┘ │From your **local machine**:# Development environment

                           └─────────────────────────────┘

```



**Components:**```bashSave and close. **Done** - you only do this once!.\scripts\deploy.ps1 -Environment dev

- **DataEngine**: Rust application processing cryptocurrency data

- **Azure Key Vault**: Encrypted secret storage with audit logsgit push origin main

- **PostgreSQL**: External database (host in Key Vault)

- **Redis**: External cache (Azure Redis Cloud)```

- **Kubernetes**: docker-desktop cluster

- **Helm**: Deployment management

- **GitHub Actions**: CI/CD on self-hosted Windows runner

The GitHub Actions workflow on your remote runner will:---# Production environment  

---

- Load secrets from `C:\secrets\dataengine\secrets.env`

## Security Features

- Build Docker image.\scripts\deploy.ps1 -Environment prod

✅ **Encryption at rest** - All secrets encrypted in Azure Key Vault  

✅ **Audit trail** - Every secret access logged in Azure  - Deploy to Kubernetes with Helm

✅ **RBAC** - Service principal has minimal permissions (get/list only)  

✅ **No plaintext storage** - Secrets never stored on runner  - Run database migrations## Deploy

✅ **Automatic rotation** - Update in Key Vault, redeploy  

✅ **Compliance** - SOC 2, ISO 27001 compliant



---### 3. Verify# With custom secrets



## Troubleshooting



### Workflow Authentication Fails```powershellFrom your local machine:.\scripts\deploy.ps1 -SecretsFile secrets.yaml



**Error:** `AADSTS7000215: Invalid client secret`# Check pods



```powershellkubectl get pods -n dataengine

# Verify GitHub Secret AZURE_CREDENTIALS is valid JSON

# Check service principal exists

az ad sp list --display-name dataengine-github-actions

# View logs```bash# Clean redeployment

# If expired, create new credentials

az ad sp credential reset --name dataengine-github-actionskubectl logs -n dataengine -l app.kubernetes.io/name=data-engine

```

git push origin main.\scripts\deploy.ps1 -Clean

### Access Denied from Key Vault

# Check health

**Error:** `The user, group or application does not have secrets get permission`

kubectl port-forward -n dataengine svc/dataengine-data-engine 8080:8080``````

```powershell

# Verify service principal has access# Then visit http://localhost:8080/health

az keyvault show --name dataengine-kv-prod --query properties.accessPolicies

```

# Grant access if missing

$SP_OBJECT_ID = az ad sp list --display-name dataengine-github-actions --query [0].id -o tsv

az keyvault set-policy --name dataengine-kv-prod --object-id $SP_OBJECT_ID --secret-permissions get list

```---GitHub Actions workflow (on your remote runner) will automatically:### Management



### Secret Not Found



**Error:** `SecretNotFound: A secret with (name/id) POSTGRES-PASSWORD was not found`## Production Setup (Azure Key Vault)- Build Docker image```powershell



```powershell

# List all secrets

az keyvault secret list --vault-name dataengine-kv-prod -o table### Why Azure Key Vault?- Deploy to Kubernetes with Helm# Check deployment status



# Add missing secret

az keyvault secret set --vault-name dataengine-kv-prod --name "POSTGRES-PASSWORD" --value "your-password"

```| Feature | File-Based (Current) | Azure Key Vault |- Run database migrations.\scripts\deploy.ps1 -Status



### Pods Won't Start|---------|---------------------|-----------------|



```powershell| Encryption | ❌ Plaintext | ✅ Encrypted at rest |

# Check pod status

kubectl describe pod -n dataengine <pod-name>| Access Control | ❌ Anyone with RDP | ✅ RBAC/Access policies |



# Check logs| Audit Trail | ❌ None | ✅ All access logged |---# View logs

kubectl logs -n dataengine <pod-name>

| Secret Rotation | ❌ Manual file edit | ✅ Automatic support |

# Check secrets created by Helm

kubectl get secrets -n dataengine| Compliance | ❌ No | ✅ SOC 2, ISO 27001 |.\scripts\logs.ps1 -Follow

kubectl describe secret postgresql-credentials -n dataengine

```| Cost | Free | ~$1/month |



### Migration Job Fails## Verify



```powershell### Setup Steps

# Check migration logs

kubectl logs -n dataengine -l app.kubernetes.io/component=migration# Health check



# Check database connectivity**Complete guide:** See `k8s/AZURE_KEYVAULT_SETUP.md`

kubectl exec -n dataengine <pod-name> -- nc -zv <postgres-host> 5432

``````powershell.\scripts\health-check.ps1 -Detailed



---**Summary:**



## Directory Structure1. Install Azure CLI on your local machine# Check deployment



```2. Create Azure Key Vault and add secrets

DataEngine/

├── .github/workflows/3. Create service principal for GitHub Actionskubectl get pods -n dataengine# Stop deployment

│   └── ci-cd.yml              # CI/CD pipeline with Azure Key Vault integration

├── k8s/4. Add GitHub Secrets: `AZURE_CREDENTIALS`, `AZURE_KEYVAULT_NAME`

│   ├── data-engine-helm/      # Helm chart

│   │   ├── Chart.yaml5. Push to deploy - workflow automatically uses Key Vault.\scripts\deploy.ps1 -Stop

│   │   ├── values.yaml

│   │   └── templates/6. Delete file on runner: `Remove-Item C:\secrets\dataengine\secrets.env`

│   └── AZURE_KEYVAULT_SETUP.md  # Complete Key Vault setup guide

├── src/                       # Rust source code# View logs```

├── Cargo.toml

├── Dockerfile**Time:** ~30 minutes  

└── README.md                  # This file

```**Cost:** ~$1/month  kubectl logs -n dataengine -l app.kubernetes.io/name=data-engine



---**Security:** Production-grade ✅



## Maintenance```### Using Makefile



### Monthly Tasks---

- [ ] Review Azure Key Vault audit logs

- [ ] Check for unused secrets```bash

- [ ] Verify service principal hasn't expired

## Update Secrets

### Quarterly Tasks (Every 90 Days)

- [ ] Rotate all secrets (database, Redis, API keys)---make dev          # Deploy development

- [ ] Review and update access policies

- [ ] Test disaster recovery procedure### Development (File-Based)



### Annual Tasks1. RDP to remote runnermake prod         # Deploy production  

- [ ] Audit all Key Vault permissions

- [ ] Review compliance requirements2. Edit: `notepad C:\secrets\dataengine\secrets.env`

- [ ] Update documentation

3. Push any commit to redeploy## Update Secretsmake status       # Check status

---



## Support

### Production (Azure Key Vault)make logs         # View logs

**Azure Key Vault Setup:** See `k8s/AZURE_KEYVAULT_SETUP.md`  

**Deployment Issues:** Check GitHub Actions logs  ```powershell

**Application Logs:** `kubectl logs -n dataengine -l app.kubernetes.io/name=data-engine`  

**Issues:** Open a GitHub issue# Update secret in Key Vault (from local machine)1. RDP to runnermake stop         # Stop deployment



---az keyvault secret set --vault-name dataengine-kv-prod --name "POSTGRES-PASSWORD" --value "new-password"



## Next Steps2. Edit: `notepad C:\secrets\dataengine\secrets.env````



1. ✅ Set up Azure Key Vault (see `k8s/AZURE_KEYVAULT_SETUP.md`)# Trigger redeployment

2. ✅ Add GitHub Secrets (`AZURE_CREDENTIALS`, `AZURE_KEYVAULT_NAME`)

3. ✅ Push to deploygit commit --allow-empty -m "Rotate secrets"3. Push any commit to redeploy

4. ⬜ Set up secret rotation schedule (every 90 days)

5. ⬜ Enable Azure Monitor for Key Vault audit logsgit push

6. ⬜ Configure alerts for failed secret access attempts

```## 🔑 Secret Management



------



## Architecture### Development (Quick Start)



```## Structure```powershell

┌─────────────────────────────────────────────────────────────┐

│                     GitHub (Push)                           │# Uses default development secrets - no setup needed

└───────────────────────────┬─────────────────────────────────┘

                            │```.\scripts\deploy.ps1

                            ▼

┌─────────────────────────────────────────────────────────────┐DataEngine/```

│           GitHub Actions (Self-Hosted Runner)               │

│  ┌──────────────────────────────────────────────────────┐   │├── k8s/

│  │  Dev: Load from C:\secrets\dataengine\secrets.env    │   │

│  │  Prod: Fetch from Azure Key Vault                    │   ││   └── data-engine-helm/     # Helm chart### Production (Secure)

│  └──────────────────────────────────────────────────────┘   │

│                            │                                 │├── src/                      # Rust source code```powershell

│                            ▼                                 │

│  ┌──────────────────────────────────────────────────────┐   │└── .github/workflows/        # CI/CD# 1. Create secrets file from template

│  │  Build Docker Image → Push to GHCR                   │   │

│  │  Deploy to Kubernetes via Helm                       │   │```Copy-Item secrets.template.yaml secrets.yaml

│  └──────────────────────────────────────────────────────┘   │

└───────────────────────────┬─────────────────────────────────┘

                            │

                            ▼---# 2. Edit with your actual credentials

┌─────────────────────────────────────────────────────────────┐

│              Kubernetes (docker-desktop)                    │notepad secrets.yaml

│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │

│  │  DataEngine  │  │  PostgreSQL  │  │    Redis     │      │## Architecture

│  │  (Rust app)  │→→│  (External)  │  │  (External)  │      │

│  └──────────────┘  └──────────────┘  └──────────────┘      │# 3. Deploy with secrets

└─────────────────────────────────────────────────────────────┘

```- **Rust**: High-performance data processing.\scripts\deploy.ps1 -Environment prod -SecretsFile secrets.yaml



**Components:**- **PostgreSQL**: External database```

- **DataEngine**: Rust application processing cryptocurrency market data

- **PostgreSQL**: External database (host TBD)- **Redis**: External cache (Azure)

- **Redis**: External cache (Azure Redis Cloud)

- **Kubernetes**: Container orchestration (docker-desktop cluster)- **Kubernetes**: Container orchestration## 🏗️ Architecture

- **Helm**: Deployment management

- **GitHub Actions**: CI/CD pipeline on self-hosted Windows runner- **Helm**: Deployment management



---- **Self-hosted runner**: Windows remote desktop- **Rust Application**: High-performance market data processing



## Troubleshooting- **PostgreSQL + TimescaleDB**: Time-series database for market data



### Development Setup---- **Redis**: High-speed caching and session storage



**Workflow fails - secrets not found:**- **Kubernetes**: Container orchestration and scaling

```powershell

# On remote runner, verify file exists## Troubleshooting- **Helm**: Configuration management and deployment

Test-Path C:\secrets\dataengine\secrets.env

- **Prometheus + Grafana**: Monitoring and observability (optional)

# Verify file has content

Get-Content C:\secrets\dataengine\secrets.env**Workflow fails:**

```

```powershell## 📊 Environments

**Workflow fails - authentication error:**

- Ensure runner has access to `C:\secrets\` directory# On runner, verify secrets file exists:

- Check file permissions

Test-Path C:\secrets\dataengine\secrets.env| Environment | Namespace | Resources | Use Case |

### Production Setup

```|-------------|-----------|-----------|----------|

**"Access denied" from Key Vault:**

```powershell| **dev** | data-engine-dev | 1 CPU, 1GB RAM | Development, testing |

# Verify service principal has access

az keyvault show --name dataengine-kv-prod --query properties.accessPolicies**Pods won't start:**| **qa** | data-engine-qa | 1 CPU, 2GB RAM | Quality assurance |

```

```powershell| **prod** | data-engine | 2 CPU, 4GB RAM | Production deployment |

**Workflow can't authenticate to Azure:**

- Verify GitHub Secret `AZURE_CREDENTIALS` is correctly formatted JSONkubectl describe pod -n dataengine <pod-name>| **monitoring** | data-engine-monitoring | + Prometheus/Grafana | Full observability |

- Check service principal exists:

  ```powershell```

  az ad sp list --display-name dataengine-github-actions

  ```## 🔧 Advanced Usage



### General Issues### Access Services

```powershell

**Pods won't start:**# DataEngine API

```powershellkubectl port-forward -n data-engine-dev service/data-engine 8080:8080

# Check pod status and events

kubectl describe pod -n dataengine <pod-name># Database (PostgreSQL)

kubectl port-forward -n data-engine-dev service/postgresql 5432:5432

# Check logs

kubectl logs -n dataengine <pod-name># Cache (Redis)

```kubectl port-forward -n data-engine-dev service/redis 6379:6379



**Migration job fails:**# Metrics (Prometheus)

```powershellkubectl port-forward -n data-engine-dev service/prometheus 9090:9090

# Check migration logs

kubectl logs -n dataengine -l app.kubernetes.io/component=migration# Dashboards (Grafana)

kubectl port-forward -n data-engine-dev service/grafana 3000:3000

# Check database connectivity```

kubectl exec -n dataengine <pod-name> -- nc -zv $POSTGRES_HOST $POSTGRES_PORT

```### Database Operations

```powershell

**Empty password in postgresql-credentials secret:**# Create backup

- This means Helm didn't receive the password.\scripts\database.ps1 -Action backup

- Verify secrets are loaded correctly in workflow

- Check GitHub Actions logs for secret loading step# Restore from backup

.\scripts\database.ps1 -Action restore

---

# Check database status

## Security Best Practices.\scripts\database.ps1 -Action status

```

### ✅ Do This

### Log Management

- **Use Azure Key Vault for production** - Encrypted, audited, compliant```powershell

- **Rotate secrets regularly** - Every 90 days minimum# Follow application logs

- **Use strong passwords** - Generated by password manager.\scripts\logs.ps1 -Follow

- **Never commit secrets** - Add `*.env`, `secrets.yaml` to `.gitignore`

- **Enable audit logging** - Monitor Key Vault access in Azure# Export logs to files

- **Principle of least privilege** - Grant minimal necessary permissions.\scripts\logs.ps1 -Export



### ❌ Don't Do This# Filter for errors

.\scripts\logs.ps1 -Filter "ERROR" -Since "1h"

- **Don't store production secrets in files** - No encryption, no audit trail```

- **Don't share secrets via Slack/email** - Use secure secret management

- **Don't use same password across environments** - Each env needs unique secrets## 🛠️ Development Workflow

- **Don't give humans direct Key Vault access** - Use RBAC roles instead

- **Don't skip secret rotation** - Old secrets = security risk```powershell

- **Don't commit secrets to Git** - Even in private repos# 1. Deploy development environment

.\scripts\deploy.ps1 -Environment dev

---

# 2. Make code changes in your IDE

## Directory Structure

# 3. Redeploy with latest changes

```.\scripts\deploy.ps1 -Clean

DataEngine/

├── .github/# 4. Monitor logs

│   └── workflows/.\scripts\logs.ps1 -Follow

│       └── ci-cd.yml                 # CI/CD pipeline (updated for Key Vault)

├── k8s/# 5. Check health

│   ├── data-engine-helm/             # Helm chart.\scripts\health-check.ps1

│   │   ├── Chart.yaml

│   │   ├── values.yaml# 6. Access application

│   │   └── templates/kubectl port-forward -n data-engine-dev service/data-engine 8080:8080

│   └── AZURE_KEYVAULT_SETUP.md       # Production secrets guide```

├── src/                              # Rust source code

├── Cargo.toml## 📚 Documentation

├── Dockerfile

└── README.md                         # This file- **Scripts**: See `scripts/README.md` for detailed script documentation

```- **Helm Chart**: See `k8s/data-engine-helm/` for Kubernetes configuration

- **API Docs**: Available at `http://localhost:8080/docs` when running

---

## 🚨 Important Notes

## Next Steps

1. **Never commit secrets**: `secrets.yaml` files should never be committed to version control

### For Development2. **Kubernetes required**: This deployment requires a running Kubernetes cluster

1. ✅ Create secrets file on remote runner3. **Resource requirements**: Ensure adequate CPU/memory for your target environment

2. ✅ Push to deploy4. **Network access**: Cluster must have internet access to pull container images

3. ✅ Verify deployment works

## 🆘 Troubleshooting

### For Production

1. ⬜ Follow `k8s/AZURE_KEYVAULT_SETUP.md`### Common Issues

2. ⬜ Set up Azure Key Vault

3. ⬜ Add GitHub Secrets**Deployment fails**:

4. ⬜ Test deployment```powershell

5. ⬜ Delete file-based secrets on runner# Check cluster connectivity

6. ⬜ Set up secret rotation schedule (every 90 days)kubectl cluster-info



---# Check pod status

kubectl get pods -n data-engine-dev

## Support

# Check events

- **Issues**: Open a GitHub issuekubectl get events -n data-engine-dev --sort-by='.lastTimestamp'

- **Documentation**: See `k8s/AZURE_KEYVAULT_SETUP.md` for production setup```

- **Logs**: `kubectl logs -n dataengine -l app.kubernetes.io/name=data-engine`

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
