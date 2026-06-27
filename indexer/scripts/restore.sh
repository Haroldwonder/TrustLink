#!/bin/bash
#
# TrustLink Indexer Database Restore Script
#
# Usage:
#   ./restore.sh <backup_file>               # Restore from file
#   ./restore.sh --list                      # List available backups
#   ./restore.sh --latest                    # Restore from most recent backup
#   ./restore.sh --s3 <s3_path>             # Restore from S3 URL
#   ./restore.sh --gcs <gcs_path>           # Restore from GCS URL
#   ./restore.sh --verify <backup_file>     # Verify backup without restoring
#   ./restore.sh --dry-run <backup_file>    # Test restore without modifying DB
#
# Environment Variables:
#   DATABASE_URL           PostgreSQL connection string (required if not using defaults)
#   BACKUP_PATH            Local backup directory (default: ./backups)
#   BACKUP_S3_BUCKET       S3 bucket name (for --list)
#   BACKUP_S3_PREFIX       S3 path prefix (default: trustlink-db-backups)
#   BACKUP_GCS_BUCKET      GCS bucket name (for --list)
#   AWS_PROFILE            AWS profile to use (default: default)
#

set -euo pipefail

# ───────────────────────────────────────────────────────────────────────────────
# Configuration
# ───────────────────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DOTENV_FILE="${PROJECT_ROOT}/.env"

# Load .env if it exists
if [[ -f "$DOTENV_FILE" ]]; then
  set +a
  source "$DOTENV_FILE"
  set -a
fi

DATABASE_URL="${DATABASE_URL:-postgresql://trustlink:trustlink@localhost:5432/trustlink}"
BACKUP_PATH="${BACKUP_PATH:-${PROJECT_ROOT}/backups}"
BACKUP_S3_BUCKET="${BACKUP_S3_BUCKET:-}"
BACKUP_S3_PREFIX="${BACKUP_S3_PREFIX:-trustlink-db-backups}"
BACKUP_GCS_BUCKET="${BACKUP_GCS_BUCKET:-}"
AWS_PROFILE="${AWS_PROFILE:-default}"

DRY_RUN="false"
VERIFY_ONLY="false"
ACTION="restore"
BACKUP_SOURCE=""

# ───────────────────────────────────────────────────────────────────────────────
# Functions
# ───────────────────────────────────────────────────────────────────────────────

log() {
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*"
}

error() {
  echo "[ERROR] $*" >&2
  exit 1
}

usage() {
  cat <<EOF
Usage: $0 [COMMAND] [OPTIONS]

Commands:
  <file>              Restore from local backup file
  --latest            Restore from most recent local backup
  --list              List available local backups
  --s3 <path>         Restore from S3 (e.g., s3://bucket/path/file.sql.gz)
  --gcs <path>        Restore from GCS (e.g., gs://bucket/path/file.sql.gz)
  --verify <file>     Verify backup integrity without restoring
  --dry-run <file>    Test restore without modifying database

Environment:
  DATABASE_URL        PostgreSQL connection string
  BACKUP_PATH         Local backup directory (default: ./backups)
  BACKUP_S3_BUCKET    S3 bucket name
  BACKUP_GCS_BUCKET   GCS bucket name
  AWS_PROFILE         AWS profile (default: default)

Example:
  ./restore.sh backups/trustlink_db_20260627_120000.sql.gz
  ./restore.sh --latest
  ./restore.sh --s3 s3://my-bucket/trustlink-db-backups/2026/06/27/trustlink_db_20260627_120000.sql.gz
EOF
  exit "${1:-0}"
}

validate_database_connection() {
  log "Validating database connection..."
  
  local db_host db_port db_user
  db_host=$(echo "$DATABASE_URL" | sed -E 's|.*@([^:]+).*|\1|')
  db_port=$(echo "$DATABASE_URL" | sed -E 's|.*:([0-9]+).*|\1|')
  db_user=$(echo "$DATABASE_URL" | sed -E 's|.*://([^:]+).*|\1|')
  
  if ! pg_isready -h "$db_host" -p "$db_port" -U "$db_user" >/dev/null 2>&1; then
    error "Database connection failed. Check DATABASE_URL: $DATABASE_URL"
  fi
  
  log "✓ Database connection OK"
}

verify_backup_file() {
  local backup_file="$1"
  
  log "Verifying backup file: $backup_file"
  
  if [[ ! -f "$backup_file" ]]; then
    error "Backup file not found: $backup_file"
  fi
  
  # Check if it's gzip compressed
  if ! file "$backup_file" | grep -q "gzip compressed"; then
    error "Backup file doesn't appear to be gzip compressed"
  fi
  
  # Check checksum if available
  local checksum_file="${backup_file}.sha256"
  if [[ -f "$checksum_file" ]]; then
    local expected_checksum actual_checksum
    expected_checksum=$(cat "$checksum_file")
    actual_checksum=$(sha256sum "$backup_file" | awk '{print $1}')
    
    if [[ "$actual_checksum" != "$expected_checksum" ]]; then
      error "Checksum mismatch: expected $expected_checksum, got $actual_checksum (file may be corrupted)"
    fi
    log "✓ Checksum verified: $expected_checksum"
  fi
  
  # Verify SQL content
  if ! gunzip -c "$backup_file" | head -100 | grep -q "CREATE TABLE\|INSERT INTO\|PostgreSQL"; then
    error "Backup file doesn't appear to contain valid PostgreSQL data"
  fi
  
  log "✓ Backup file verified"
  return 0
}

download_from_s3() {
  local s3_path="$1"
  local output_file
  output_file=$(mktemp)
  
  log "Downloading backup from S3: $s3_path"
  
  if ! aws s3 cp "$s3_path" "$output_file" --profile "$AWS_PROFILE"; then
    error "Failed to download from S3"
  fi
  
  echo "$output_file"
}

download_from_gcs() {
  local gcs_path="$1"
  local output_file
  output_file=$(mktemp)
  
  log "Downloading backup from GCS: $gcs_path"
  
  if ! gsutil cp "$gcs_path" "$output_file"; then
    error "Failed to download from GCS"
  fi
  
  echo "$output_file"
}

list_local_backups() {
  log "Local backups in $BACKUP_PATH:"
  
  if [[ ! -d "$BACKUP_PATH" ]] || [[ -z "$(ls -1 "$BACKUP_PATH"/*.sql.gz 2>/dev/null)" ]]; then
    log "  (none)"
    return
  fi
  
  ls -lht "$BACKUP_PATH"/trustlink_db_*.sql.gz 2>/dev/null | awk '{
    print "  " $9 " (" $5 " bytes, " $6 " " $7 " " $8 ")"
  }'
}

list_s3_backups() {
  if [[ -z "$BACKUP_S3_BUCKET" ]]; then
    error "BACKUP_S3_BUCKET not set"
  fi
  
  log "S3 backups in s3://${BACKUP_S3_BUCKET}/${BACKUP_S3_PREFIX}:"
  
  aws s3 ls "s3://${BACKUP_S3_BUCKET}/${BACKUP_S3_PREFIX}/" \
    --recursive \
    --profile "$AWS_PROFILE" | grep "\.sql\.gz$" | awk '{
    print "  " $4 " (" $3 " bytes, " $1 " " $2 ")"
  }' || log "  (none or error reading S3)"
}

list_gcs_backups() {
  if [[ -z "$BACKUP_GCS_BUCKET" ]]; then
    error "BACKUP_GCS_BUCKET not set"
  fi
  
  log "GCS backups in gs://${BACKUP_GCS_BUCKET}:"
  
  gsutil ls -h "gs://${BACKUP_GCS_BUCKET}/trustlink-db-backups/" 2>/dev/null | grep "\.sql\.gz$" || log "  (none or error reading GCS)"
}

get_latest_backup() {
  if [[ ! -d "$BACKUP_PATH" ]]; then
    error "Backup directory not found: $BACKUP_PATH"
  fi
  
  local latest
  latest=$(ls -1t "$BACKUP_PATH"/trustlink_db_*.sql.gz 2>/dev/null | head -1)
  
  if [[ -z "$latest" ]]; then
    error "No backups found in $BACKUP_PATH"
  fi
  
  echo "$latest"
}

restore_database() {
  local backup_file="$1"
  
  log "Starting database restore from: $backup_file"
  
  if [[ "$DRY_RUN" == "true" ]]; then
    log "[DRY-RUN] Would restore from: $backup_file"
    log "[DRY-RUN] Database would be restored to: $DATABASE_URL"
    return 0
  fi
  
  # Create a backup of current state before restoring
  log "Creating safety backup before restore..."
  local safety_backup
  safety_backup="${BACKUP_PATH}/pre_restore_backup_$(date +%Y%m%d_%H%M%S).sql.gz"
  
  if ! pg_dump "$DATABASE_URL" 2>/dev/null | gzip -9 > "$safety_backup"; then
    error "Failed to create safety backup"
  fi
  log "✓ Safety backup created: $safety_backup"
  
  # Kill all connections except current one
  log "Terminating other database connections..."
  psql "$DATABASE_URL" -c "
    SELECT pg_terminate_backend(pg_stat_activity.pid)
    FROM pg_stat_activity
    WHERE pg_stat_activity.datname = current_database()
      AND pid <> pg_backend_pid()
    " || log "⚠ Warning: some connections couldn't be terminated"
  
  # Drop and recreate database
  log "Dropping current database..."
  local db_name db_host db_user db_port
  db_name=$(echo "$DATABASE_URL" | sed -E 's|.*/([^?]+).*|\1|')
  db_host=$(echo "$DATABASE_URL" | sed -E 's|.*@([^:]+).*|\1|')
  db_port=$(echo "$DATABASE_URL" | sed -E 's|.*:([0-9]+).*|\1|')
  db_user=$(echo "$DATABASE_URL" | sed -E 's|.*://([^:]+).*|\1|')
  
  # Connect to template1 to drop the database
  local template_url="postgresql://${db_user}:$(echo "$DATABASE_URL" | sed -E 's|.*:([^@]+)@.*|\1|')@${db_host}:${db_port}/template1"
  
  if ! dropdb --if-exists "$db_name" --host="$db_host" --port="$db_port" --username="$db_user" 2>/dev/null; then
    log "⚠ Warning: couldn't drop database (may already be dropped)"
  fi
  
  log "Creating new database..."
  if ! createdb "$db_name" --host="$db_host" --port="$db_port" --username="$db_user"; then
    error "Failed to create database. Restore aborted. Safety backup saved at: $safety_backup"
  fi
  
  # Restore backup
  log "Restoring database..."
  if ! gunzip -c "$backup_file" | psql "$DATABASE_URL" >/dev/null 2>&1; then
    error "Restore failed. Database may be in inconsistent state. Safety backup at: $safety_backup"
  fi
  
  log "✓ Restore completed successfully"
  log "  Safety backup: $safety_backup"
}

# ───────────────────────────────────────────────────────────────────────────────
# Main
# ───────────────────────────────────────────────────────────────────────────────

main() {
  if [[ $# -eq 0 ]]; then
    usage 1
  fi
  
  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help|-h)
        usage 0
        ;;
      --list)
        list_local_backups
        list_s3_backups 2>/dev/null || true
        list_gcs_backups 2>/dev/null || true
        exit 0
        ;;
      --latest)
        BACKUP_SOURCE=$(get_latest_backup)
        shift
        ;;
      --s3)
        if [[ -z "${2:-}" ]]; then
          error "--s3 requires an S3 path argument"
        fi
        BACKUP_SOURCE=$(download_from_s3 "$2")
        trap "rm -f $BACKUP_SOURCE" EXIT
        shift 2
        ;;
      --gcs)
        if [[ -z "${2:-}" ]]; then
          error "--gcs requires a GCS path argument"
        fi
        BACKUP_SOURCE=$(download_from_gcs "$2")
        trap "rm -f $BACKUP_SOURCE" EXIT
        shift 2
        ;;
      --verify)
        if [[ -z "${2:-}" ]]; then
          error "--verify requires a backup file argument"
        fi
        VERIFY_ONLY="true"
        BACKUP_SOURCE="$2"
        shift 2
        ;;
      --dry-run)
        if [[ -z "${2:-}" ]]; then
          error "--dry-run requires a backup file argument"
        fi
        DRY_RUN="true"
        BACKUP_SOURCE="$2"
        shift 2
        ;;
      *)
        BACKUP_SOURCE="$1"
        shift
        ;;
    esac
  done
  
  if [[ -z "$BACKUP_SOURCE" ]]; then
    error "No backup source specified"
  fi
  
  log "TrustLink Database Restore Script"
  
  verify_backup_file "$BACKUP_SOURCE"
  
  if [[ "$VERIFY_ONLY" == "true" ]]; then
    log "✓ Backup verified successfully"
    exit 0
  fi
  
  validate_database_connection
  
  log "⚠ WARNING: This will restore the database from: $BACKUP_SOURCE"
  log "⚠ All current data will be replaced"
  read -p "Continue? (type 'yes' to confirm): " -r confirmation
  
  if [[ "$confirmation" != "yes" ]]; then
    log "Restore cancelled"
    exit 0
  fi
  
  restore_database "$BACKUP_SOURCE"
}

main "$@"
