# DataEngine CI/CD

**Ultra-automated deployment pipeline: Push code → Build image → Deploy to Kubernetes**

---

## 🚀 How It Works

### Automatic Deployment (99% of the time)

```bash
# Make your changes
git add .
git commit -m "Add new feature"
git push

# That's it! GitHub Actions automatically:
# ✅ Builds Docker image
# ✅ Pushes to ghcr.io/nwagbara-group-llc/dataengine:latest
# ✅ Deploys PostgreSQL (if needed)
# ✅ Deploys Redis (if needed)
# ✅ Runs database migrations
# ✅ Deploys DataEngine with new image
# ✅ Verifies everything is running
```

**Watch progress:** https://github.com/Nwagbara-Group-LLC/DataEngine/actions

---

## 🔧 Manual Deployment (Optional)

### Clean Deploy (Start Fresh)

If you need to wipe everything and start from scratch:

1. Go to: https://github.com/Nwagbara-Group-LLC/DataEngine/actions
2. Click "CI/CD - Build and Deploy DataEngine"
3. Click "Run workflow"
4. ✅ Check "Delete all resources and redeploy from scratch"
5. Click "Run workflow"

This will:
- Delete the entire `dataengine` namespace
- Recreate everything from scratch
- Run fresh migrations
- Deploy clean DataEngine

---

## 📋 Deployment Flow

```
┌─────────────────────────────────────────────────────────────┐
│  PHASE 1: BUILD                                             │
├─────────────────────────────────────────────────────────────┤
│  ✅ Checkout code                                           │
│  ✅ Login to GHCR                                           │
│  ✅ Build Docker image                                      │
│  ✅ Push image (tags: latest, sha-abc1234)                 │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│  PHASE 2: PREPARE                                           │
├─────────────────────────────────────────────────────────────┤
│  ✅ Create namespace (dataengine)                           │
│  ✅ Create secrets (database, redis, GHCR pull)            │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│  PHASE 3: INFRASTRUCTURE                                    │
├─────────────────────────────────────────────────────────────┤
│  ✅ Deploy PostgreSQL (wait for ready)                     │
│  ✅ Deploy Redis (wait for ready)                          │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│  PHASE 4: DATABASE                                          │
├─────────────────────────────────────────────────────────────┤
│  ✅ Delete old migration job                               │
│  ✅ Run new migrations (wait for completion)               │
│  ✅ Show migration logs                                     │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│  PHASE 5: APPLICATION                                       │
├─────────────────────────────────────────────────────────────┤
│  ✅ Deploy DataEngine (wait for ready)                     │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│  PHASE 6: VERIFY                                            │
├─────────────────────────────────────────────────────────────┤
│  ✅ Show pods, services, deployments                       │
│  ✅ Show recent logs                                        │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔍 Monitoring Deployment

### Check Status
```powershell
# All resources in dataengine namespace
kubectl get all -n dataengine

# Pod status
kubectl get pods -n dataengine -w

# Logs
kubectl logs -f deployment/dataengine -n dataengine
```

### Access DataEngine
```powershell
# Port forward to access locally
kubectl port-forward -n dataengine svc/dataengine 8080:8080

# Then open: http://localhost:8080
```

### Database Access
```powershell
# Connect to PostgreSQL
kubectl exec -it -n dataengine statefulset/postgresql -- psql -U dataengine -d dataengine

# Connect to Redis
kubectl exec -it -n dataengine statefulset/redis -- redis-cli
```

---

## 📂 Kubernetes Manifests

All deployment files are in `k8s/simple/`:

| File | Purpose |
|------|---------|
| `postgresql.yaml` | PostgreSQL StatefulSet (official postgis/postgis:15-3.3 image) |
| `redis.yaml` | Redis StatefulSet (official redis:7-alpine image) |
| `migrations.yaml` | Database migration Job (uses databaseschema image) |
| `dataengine.yaml` | DataEngine Deployment (2 replicas, auto-scaling ready) |

**Note:** These use **official images** for PostgreSQL and Redis - no custom builds needed!

---

## 🔐 Secrets Required

The workflow needs these GitHub secrets (set in repository settings):

| Secret | Required | Default | Purpose |
|--------|----------|---------|---------|
| `PERSONAL_ACCESS_TOKEN` | ✅ Yes | - | GHCR login and image pull |
| `POSTGRES_PASSWORD` | No | `dataengine123` | PostgreSQL password |
| `REDIS_PASSWORD` | No | `redis123` | Redis password |

**Set secrets:** https://github.com/Nwagbara-Group-LLC/DataEngine/settings/secrets/actions

---

## 🐛 Troubleshooting

### Build Fails
```powershell
# Check GitHub Actions logs
# Common issues:
# - Dependency compilation errors
# - Dockerfile path problems
# - GHCR login failures

# Fix: Update Dockerfile or dependencies in Cargo.toml
```

### PostgreSQL Won't Start
```powershell
# Check pod status
kubectl describe pod -n dataengine -l app=postgresql

# Check logs
kubectl logs -n dataengine statefulset/postgresql

# Common issues:
# - PersistentVolume not available
# - Secret missing
# - Port conflicts
```

### Migrations Fail
```powershell
# Check migration job logs
kubectl logs -n dataengine job/database-migrations

# Delete and retry
kubectl delete job database-migrations -n dataengine
# Push code again to trigger workflow
```

### DataEngine Pods CrashLooping
```powershell
# Check logs
kubectl logs -n dataengine deployment/dataengine --previous

# Common issues:
# - Can't connect to database
# - Can't connect to Redis
# - Missing environment variables
# - Health check failing
```

### Force Redeploy
```powershell
# Option 1: Use clean deploy (GitHub UI)
# Option 2: Manual restart
kubectl rollout restart deployment/dataengine -n dataengine

# Option 3: Delete and let workflow recreate
kubectl delete deployment dataengine -n dataengine
# Push code to trigger workflow
```

---

## 🎯 Next Steps After First Deployment

1. **Verify everything is running:**
   ```powershell
   kubectl get all -n dataengine
   ```

2. **Check DataEngine logs:**
   ```powershell
   kubectl logs -f deployment/dataengine -n dataengine
   ```

3. **Test the API:**
   ```powershell
   kubectl port-forward -n dataengine svc/dataengine 8080:8080
   # Visit http://localhost:8080
   ```

4. **Monitor ongoing:**
   - Watch GitHub Actions for each push
   - Check pod health: `kubectl get pods -n dataengine -w`

---

## 📝 Development Workflow

```bash
# 1. Create feature branch
git checkout -b feature/new-data-source

# 2. Make changes to DataEngine code
# Edit files in DataEngine/

# 3. Commit and push
git add .
git commit -m "Add new data source"
git push origin feature/new-data-source

# 4. Create PR, review, merge to main

# 5. Automatic deployment happens on merge!
```

---

**Built with:** Rust 🦀 | Docker 🐳 | Kubernetes ☸️ | GitHub Actions 🤖
