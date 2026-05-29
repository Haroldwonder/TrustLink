variable "aws_region" {
  description = "AWS region to deploy into"
  type        = string
  default     = "us-east-1"
}

variable "environment" {
  description = "Deployment environment: testnet or mainnet"
  type        = string
  validation {
    condition     = contains(["testnet", "mainnet"], var.environment)
    error_message = "environment must be 'testnet' or 'mainnet'."
  }
}

variable "name_prefix" {
  description = "Prefix applied to all resource names"
  type        = string
  default     = "trustlink"
}

# ── Database ──────────────────────────────────────────────────────────────────

variable "db_instance_class" {
  description = "RDS instance class"
  type        = string
  default     = "db.t3.micro"
}

variable "db_allocated_storage" {
  description = "RDS allocated storage in GB"
  type        = number
  default     = 20
}

variable "db_username" {
  description = "PostgreSQL master username"
  type        = string
  default     = "trustlink"
}

variable "db_password" {
  description = "PostgreSQL master password"
  type        = string
  sensitive   = true
}

# ── Indexer (ECS/Fargate) ─────────────────────────────────────────────────────

variable "indexer_image" {
  description = "Docker image for the indexer (e.g. ghcr.io/org/trustlink-indexer:latest)"
  type        = string
}

variable "indexer_port" {
  description = "Port the indexer container listens on"
  type        = number
  default     = 4000
}

variable "indexer_cpu" {
  description = "Fargate task CPU units"
  type        = number
  default     = 256
}

variable "indexer_memory" {
  description = "Fargate task memory in MiB"
  type        = number
  default     = 512
}

variable "indexer_desired_count" {
  description = "Number of indexer tasks to run"
  type        = number
  default     = 1
}

# ── Stellar ───────────────────────────────────────────────────────────────────

variable "stellar_network" {
  description = "Stellar network passphrase or name (testnet / mainnet)"
  type        = string
  default     = "testnet"
}

variable "contract_id" {
  description = "Deployed TrustLink contract ID"
  type        = string
}
