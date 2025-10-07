# Trading Platform Architecture Decisions

## Log Aggregation: Filebeat vs Logstash

**Decision: Use Filebeat**

### Why Filebeat?

1. **Lightweight**: 10-50 MB RAM vs Logstash's 500 MB - 2 GB
2. **Your logs are already structured**: ultra-logger outputs clean JSON
3. **No heavy parsing needed**: Trading data doesn't need grok patterns or complex ETL
4. **Container-friendly**: Sidecar/DaemonSet pattern perfect for Kubernetes
5. **Direct to Elastic Cloud**: Efficient shipping without intermediate hops

### Why NOT Logstash?

- **Overkill for structured logs**: Logstash is for unstructured text that needs heavy transformation
- **Resource waste**: ~500 MB per instance adds significant overhead in trading platform
- **Added complexity**: Another component to manage, debug, and scale
- **Unnecessary latency**: Extra hop in log pipeline

### Architecture

```
Trading Services (DataEngine, SignalEngine, etc.)
    ↓
ultra-logger library (imported from LoggingEngine)
    ↓
Structured JSON logs written to stdout/files
    ↓
Filebeat (DaemonSet watches all pods)
    ↓
Elastic Cloud
    ↓
Kibana dashboards (kibana/dashboards/) + Alerts (kibana/alerts/)
```

## Library vs Service Clarity

### The Problem (Before)

Three repositories were confused about their purpose:

1. **databaseschema**: Had Dockerfile, workflows, Helm chart - but it's a library!
2. **redisutils**: Same bloat - Dockerfile for a library with no binary
3. **LoggingEngine**: HTTP service wrapper duplicating Filebeat

This led to:
- ~1000 lines of unnecessary deployment code
- Overcomplicated workflows (543-line deploy.yml)
- Architectural confusion (trying to deploy libraries as services)
- Wasted CI/CD resources building/pushing unused images

### The Solution (After)

**Clear Separation:**

| Repository | Type | Deployable? | Purpose |
|------------|------|-------------|---------|
| **databaseschema** | Library + Migrations | Job only | Diesel migrations runner |
| **redisutils** | Library | No | Redis connection pool utilities |
| **ultra-logger** | Library | No | Logging library imported by services |
| **DataEngine** | Service | Yes | Main trading application |
| **SignalEngine** | Service | Yes | Signal processing service |
| **PostgreSQL** | Official Image | Yes | postgis/postgis:15-3.3 |
| **Redis** | Official Image | Yes | redis:7-alpine |

### Impact

- **Lines removed**: ~1400 (400 from LoggingEngine, 700+ from databaseschema, 300+ from redisutils)
- **Docker images eliminated**: 3 (logging-engine-service, databaseschema fake server, redisutils)
- **Workflows simplified**: 543 lines → 80 lines (deploy-correct.yml)
- **Clarity gained**: Architecture is now obvious from repository structure

## Database & Redis Deployment

**Decision: Use official images directly, not Helm subchart dependencies**

### Why?

1. **Avoid Bitnami bloat**: Helm automatically tries to use Bitnami charts for `postgresql`/`redis` keys
2. **Explicit control**: StatefulSets with official images are predictable
3. **Migration separation**: Diesel migrations run in separate Job using databaseschema image
4. **Industry standard**: postgis/postgis:15-3.3 and redis:7-alpine are battle-tested

### Structure

```
k8s/simple/
├── postgresql.yaml    # StatefulSet with postgis/postgis:15-3.3
├── redis.yaml         # StatefulSet with redis:7-alpine
└── migrations.yaml    # Job running diesel migration run
```

## GitHub Actions Deployment

**Decision: Simple manifests over fighting Helm**

### Before (deploy.yml - 543 lines)

- Nuclear namespace deletion on every deploy
- Pre-create ServiceAccount workarounds
- 20+ DEBUG sections
- Fighting Helm's implicit Bitnami support
- Mixing databases with application deployment

### After (deploy-correct.yml - 80 lines)

- Apply PostgreSQL StatefulSet (official image)
- Apply Redis StatefulSet (official image)
- Run migration Job
- Use Helm ONLY for DataEngine application
- Clean separation of concerns

### Philosophy

**Use the right tool:**
- **Helm**: For your custom applications (DataEngine, SignalEngine)
- **StatefulSets**: For stateful infrastructure (PostgreSQL, Redis)
- **Jobs**: For one-time operations (migrations)
- **Official images**: For databases and caches

**Don't use Helm for:**
- Databases (use official images in StatefulSets)
- Libraries (they're imported, not deployed)
- Things that don't need deployment (utils, shared code)

## Lessons Learned

1. **Libraries aren't services**: If it has no `main.rs` or binary, it doesn't need a Dockerfile
2. **Question assumptions**: "Why are we using Bitnami?" led to discovering it was implicit
3. **Simpler is better**: 80 lines beats 543 when both achieve the same result
4. **Official > Custom**: PostgreSQL official image > our wrapper > Bitnami chart
5. **Separation of concerns**: Databases, migrations, and applications are different lifecycle stages

## Next Steps

1. ✅ **Completed**: Cleaned databaseschema, redisutils, LoggingEngine
2. ✅ **Completed**: Created simple deployment manifests
3. ⏳ **Testing**: Deploy with deploy-correct.yml
4. ⏳ **Fix**: GHCR authentication (403 Forbidden on private packages)
5. ⏳ **Cleanup**: Delete deploy.yml after successful test
6. 🔮 **Future**: Apply same pattern to other services (SignalEngine, etc.)

## References

- `CLEANUP-SUMMARY.md` in databaseschema
- `README-NEW.md` in redisutils
- `CLEANUP-SUMMARY.md` in LoggingEngine
- `ARCHITECTURE-EXPLAINED.txt` in DataEngine
- `deploy-correct.yml` - The simple approach

---

**Author**: Cleanup performed October 6, 2025  
**Context**: Refactoring from overcomplicated Helm/Docker setup to clean separation of libraries vs services
