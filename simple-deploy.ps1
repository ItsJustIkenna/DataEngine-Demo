# Option 2: Simple approach - Deploy databases separately

# Deploy PostgreSQL (using Bitnami chart - it works!)
helm upgrade --install postgres oci://registry-1.docker.io/bitnamicharts/postgresql `
  --namespace dataengine `
  --create-namespace `
  --set auth.username=dataengine `
  --set auth.password=dataengine123 `
  --set auth.database=dataengine `
  --set primary.persistence.size=10Gi

# Deploy Redis (using Bitnami chart)  
helm upgrade --install redis oci://registry-1.docker.io/bitnamicharts/redis `
  --namespace dataengine `
  --set auth.enabled=false `
  --set master.persistence.size=5Gi

# Deploy YOUR app (simple chart, no database management)
helm upgrade --install dataengine ./k8s/data-engine-helm `
  --namespace dataengine `
  --set postgresql.enabled=false `
  --set redis.enabled=false `
  --set postgresql.host=postgres-postgresql `
  --set redis.host=redis-master
