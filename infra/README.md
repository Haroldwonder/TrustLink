# TrustLink Infrastructure (Terraform)

Infrastructure-as-code for deploying TrustLink indexer, database, and monitoring on AWS.

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│ CloudFront / ALB (Public)                           │
└─────────────────┬───────────────────────────────────┘
                  │
┌─────────────────┴───────────────────────────────────┐
│ ECS Fargate Cluster                                 │
│ ├─ Indexer Service (1+ tasks)                       │
│ └─ Auto-scaling (if configured)                     │
└──────────────────────┬────────────────────────────────┘
                       │
┌──────────────────────┴────────────────────────────────┐
│ RDS PostgreSQL Instance                              │
│ └─ Backups, failover, encryption at rest             │
└───────────────────────────────────────────────────────┘
```

## Prerequisites

- Terraform >= 1.5
- AWS credentials configured locally (`~/.aws/credentials` or env vars)
- Deployed TrustLink contract (have the contract ID ready)

## Deployment

### 1. Initialize Terraform

```bash
cd infra/terraform

# Download providers and modules
terraform init
```

### 2. Create `terraform.tfvars`

Copy and customize a `.tfvars` file for your environment:

```bash
# For testnet
cp testnet.tfvars terraform.tfvars
# or for mainnet
cp mainnet.tfvars terraform.tfvars
```

Then edit to fill in required variables:

```hcl
environment = "testnet"
aws_region  = "us-east-1"
db_password = "your-secure-password"
contract_id = "CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
alert_email = "ops@yourdomain.com"
```

### 3. Plan the Deployment

```bash
terraform plan -out=tfplan
```

Review the resource summary to ensure it matches your intentions.

### 4. Apply the Deployment

```bash
terraform apply tfplan
```

This will:
- Create VPC security groups
- Deploy RDS PostgreSQL instance
- Create ECS cluster and Fargate task definition
- Set up Application Load Balancer (ALB)
- Configure AWS Budgets for cost monitoring

### 5. Retrieve Outputs

After deployment, view the outputs:

```bash
terraform output
```

Key outputs:
- `alb_dns_name` — Your indexer's public DNS endpoint (HTTP)
- `rds_endpoint` — PostgreSQL connection host
- `ecs_cluster_name` — ECS cluster name
- `budget_alert_topic_arn` — SNS topic for cost alerts

## Cost Monitoring & Budget Alerts

### Setup

Cost monitoring is configured automatically by the Terraform module:

1. **AWS Budgets** — Monthly budget tracking at the specified threshold (default: $100 USD)
2. **SNS Notifications** — Email alerts when forecasted spend reaches 90% of budget

The email provided in `alert_email` will receive:
- **Subscription confirmation** email — confirm subscription to budget alerts
- **Cost forecasts** — when monthly spend is projected to exceed the threshold

### Configuration

Edit budget settings in `terraform.tfvars`:

```hcl
budget_threshold = 150  # Alert when monthly spend reaches $150 USD
alert_email      = "ops@example.com"
```

Then re-apply:

```bash
terraform apply
```

### Monitoring Costs

View detailed costs in the AWS console:

1. **AWS Budgets** — https://console.aws.amazon.com/billing/home#/budgets
2. **Cost Explorer** — https://console.aws.amazon.com/cost-management/home#/custom
3. **Billing Dashboard** — https://console.aws.amazon.com/billing/home

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `environment` | Yes | `testnet` or `mainnet` |
| `aws_region` | No | AWS region (default: `us-east-1`) |
| `db_password` | Yes | PostgreSQL master password (sensitive) |
| `contract_id` | Yes | Deployed contract address |
| `alert_email` | Yes | Email for cost alerts |
| `budget_threshold` | No | Monthly budget USD (default: 100) |
| `db_instance_class` | No | RDS instance type (default: `db.t3.micro`) |
| `indexer_cpu` | No | Fargate task CPU units (default: 256) |
| `indexer_memory` | No | Fargate task memory MiB (default: 512) |

## Connecting the Indexer

Once deployed, configure your indexer environment variables:

```bash
export DATABASE_URL="postgresql://trustlink:PASSWORD@RDS_ENDPOINT:5432/trustlink"
export CONTRACT_ID="your-contract-id"
export PORT=3000
```

Then deploy the indexer Docker image to ECS via the Terraform `indexer_image` variable, or update it directly:

```bash
# Update ECS service with new indexer image
aws ecs update-service \
  --cluster trustlink-cluster \
  --service trustlink-indexer \
  --force-new-deployment \
  --region us-east-1
```

## Scaling

### Database (RDS)

To upgrade the database instance:

```hcl
db_instance_class = "db.t3.small"  # Larger instance type
db_allocated_storage = 100          # Increase storage (GB)
```

Apply:

```bash
terraform apply
```

⚠️ **Caution:** Changing instance class causes a brief downtime. Schedule during a maintenance window.

### Compute (ECS)

To scale indexer tasks:

```hcl
indexer_desired_count = 2  # Run 2 tasks behind ALB load balancer
indexer_cpu = 512          # More CPU per task
indexer_memory = 1024      # More memory per task
```

Apply:

```bash
terraform apply
```

## Destroying Infrastructure

⚠️ **Caution:** This permanently deletes all resources.

```bash
terraform destroy
```

Confirm when prompted.

## Troubleshooting

### Database Connection Issues

Check security groups:

```bash
aws ec2 describe-security-groups --filters "Name=group-name,Values=trustlink-rds" --region us-east-1
```

Verify ECS task can reach PostgreSQL:

```bash
aws ecs execute-command \
  --cluster trustlink-cluster \
  --task <TASK_ID> \
  --container indexer \
  --interactive \
  --command "/bin/sh" \
  --region us-east-1

# Inside the task:
curl -v postgresql://trustlink:PASSWORD@rds-endpoint:5432/trustlink
```

### Indexer Logs

View recent logs:

```bash
aws logs tail /ecs/trustlink-indexer --follow --region us-east-1
```

### Budget Alert Emails Not Received

1. Check SNS subscription confirmation was accepted
2. Verify email address is correct in `alert_email`
3. Check AWS Budgets dashboard for alert status

## Maintenance

### Database Backups

RDS is configured with automatic backups (7-day retention). For mainnet, deletion protection is enabled.

To manually create a snapshot:

```bash
aws rds create-db-snapshot \
  --db-instance-identifier trustlink-indexer \
  --db-snapshot-identifier trustlink-indexer-backup-$(date +%s) \
  --region us-east-1
```

### Logs Retention

ECS logs are retained for **30 days** by default. Update in `main.tf`:

```hcl
retention_in_days = 90  # Retain longer
```

## Security Best Practices

1. **Never commit `terraform.tfvars`** — add to `.gitignore`
2. **Use AWS Secrets Manager** for sensitive values
3. **Enable encryption** on RDS and EBS
4. **Restrict ALB security group** to known IPs if possible
5. **Regular key rotation** for database credentials
6. **Monitor CloudTrail logs** for infrastructure changes

## Support

For issues or questions, open a GitHub issue on the TrustLink repository.
