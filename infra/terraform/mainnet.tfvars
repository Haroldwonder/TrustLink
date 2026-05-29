environment     = "mainnet"
aws_region      = "us-east-1"
name_prefix     = "trustlink-mainnet"
stellar_network = "mainnet"

db_instance_class    = "db.t3.small"
db_allocated_storage = 50
indexer_desired_count = 2

# db_password  = set via TF_VAR_db_password env var
# contract_id  = set via TF_VAR_contract_id env var
# indexer_image = "ghcr.io/your-org/trustlink-indexer:latest"
