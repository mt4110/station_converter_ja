# DEPLOY

## Policy

- AWS first
- GCP / Azure skeleton kept in-tree
- keep HCL plain
- allow local `tofu` / CI `terraform-compatible` execution
- app containers stay portable

## Targets

### AWS
- ECS/Fargate or App Runner
- RDS PostgreSQL or Aurora PostgreSQL
- ElastiCache Redis
- S3 for SQLite artifact publishing

### GCP
- Cloud Run
- Cloud SQL PostgreSQL
- Memorystore Redis
- Cloud Storage for SQLite artifact publishing

### Azure
- Container Apps
- Azure Database for PostgreSQL
- Azure Cache for Redis
- Blob Storage for SQLite artifact publishing

## Notes

This scaffold only ships the directory layout and variable contracts.
Real cloud resources are not implemented yet.
