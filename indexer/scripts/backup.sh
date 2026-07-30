#!/bin/bash
#
# TrustLink Indexer Database Backup Script
#
# Usage:
#   ./backup.sh                    # Use .env defaults
#   ./backup.sh --local            # Backup to local directory
#   ./backup.sh --s3               # Backup to S3
#   ./backup.sh --gcs              # Backup to Google Cloud Storage
#   ./backup.sh --dry-run          # Test without uploading
#
# Environment Variables:
#   DATABASE_URL           PostgreSQL connection string (required if not using defaults)
#   BACKUP_STORAGE         Storage destination: local|s3|gcs (default: local)
#   BACKUP_PATH            Local backup directory (default: ./backups)
#   BACKUP_S3_BUCKET       S3 bucket name (required for S3 backups)
#   BACKUP_S3_PREFIX       S3 path prefix (default: trustlink-db-backups)
#   BACKUP_GCS_BUCKET      GCS bucket name (required for GCS backups)
#   BACKUP_RETENTION_DAYS  Keep backups for N days (default: 30)
#   AWS_PROFILE            AWS profile to use (default: default)
#   GOOGLE_CLOUD_PROJECT   GCP project ID (required for GCS)
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

# Defaults
DATABASE_URL="${DATABASE_URL:-postgresql://trustlink:trustlink@localhost:5432/trustlink}"
BACKUP_STORAGE="${BACKUP_STORAGE:-local}"
BACKUP_PATH="${BACKUP_PATH:-${PROJECT_ROOT}/backups}"
BACKUP_S3_BUCKET="${BACKUP_S3_BUCKET:-}"
BACKUP_S3_PREFIX="${BACKUP_S3_PREFIX:-trustlink-db-backups}"
BACKUP_GCS_BUCKET="${BACKUP_GCS_BUCKET:-}"
BACKUP_RETENTION_DAYS="${BACKUP_RETENTION_DAYS:-30}"
AWS_PROFILE="${AWS_PROFILE:-default}"
DRY_RUN="${DRY_RUN:-false}"

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --local)
      BACKUP_STORAGE="local"
      shift
      ;;
    --s3)
      BACKUP_STORAGE="s3"
      shift
      ;;
    --gcs)
      BACKUP_STORAGE="gcs"
      shift
      ;;
    --dry-run)
      DRY_RUN="true"
      shift
      ;;
    *)
      echo "Unknown argument: $1"
      exit 1
      ;;
  esac
done

# Timestamp for backup file
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILENAME="trustlink_db_${TIMESTAMP}.sql.gz"
BACKUP_CHECKSUM_FILE="trustlink_db_${TIMESTAMP}.sql.gz.sha256"

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

validate_database_connection() {
  log "Validating database connection..."
  
  # Extract connection parameters
  local db_host db_port db_user db_name
  db_host=$(echo "$DATABASE_URL" | sed -E 's|.*@([^:]+).*|\1|')
  db_port=$(echo "$DATABASE_URL" | sed -E 's|.*:([0-9]+).*|\1|')
  db_user=$(echo "$DATABASE_URL" | sed -E 's|.*://([^:]+).*|\1|')
  db_name=$(echo "$DATABASE_URL" | sed -E 's|.*/([^?]+).*|\1|')
  
  if ! pg_isready -h "$db_host" -p "$db_port" -U "$db_user" >/dev/null 2>&1; then
    error "Database connection failed. Check DATABASE_URL: $DATABASE_URL"
  fi
  
  log "✓ Database connection OK"
}

backup_database() {
  log "Starting database backup..."
  
  local temp_dir
  temp_dir=$(mktemp -d)
  local sql_file="${temp_dir}/trustlink_db_${TIMESTAMP}.sql"
  
  trap "rm -rf $temp_dir" EXIT
  
  # Create backup
  if ! pg_dump "$DATABASE_URL" | gzip -9 > "${sql_file}.gz"; then
    error "pg_dump failed"
  fi
  
  log "✓ Database backup created: $(basename ${sql_file}.gz)"
  
  # Calculate checksum
  local file_size checksum
  file_size=$(stat -f%z "${sql_file}.gz" 2>/dev/null || stat -c%s "${sql_file}.gz")
  checksum=$(sha256sum "${sql_file}.gz" | awk '{print $1}')
  
  log "  Size: $(numfmt --to=iec $file_size 2>/dev/null || printf '%d bytes' $file_size)"
  log "  SHA256: $checksum"
  
  # Store backup and checksum in temp location
  echo "$checksum" > "${temp_dir}/${BACKUP_CHECKSUM_FILE}"
  
  echo "$sql_file" "$checksum" "$file_size"
}

upload_to_local() {
  local sql_file="$1"
  local checksum="$2"
  local file_size="$3"
  
  log "Uploading backup to local storage: $BACKUP_PATH"
  
  mkdir -p "$BACKUP_PATH"
  
  if [[ "$DRY_RUN" != "true" ]]; then
    cp "${sql_file}.gz" "${BACKUP_PATH}/${BACKUP_FILENAME}"
    echo "$checksum" > "${BACKUP_PATH}/${BACKUP_CHECKSUM_FILE}"
    log "✓ Backup saved to: ${BACKUP_PATH}/${BACKUP_FILENAME}"
  else
    log "[DRY-RUN] Would save to: ${BACKUP_PATH}/${BACKUP_FILENAME}"
  fi
}

upload_to_s3() {
  local sql_file="$1"
  local checksum="$2"
  local file_size="$3"
  
  if [[ -z "$BACKUP_S3_BUCKET" ]]; then
    error "BACKUP_S3_BUCKET is required for S3 uploads"
  fi
  
  local s3_path="s3://${BACKUP_S3_BUCKET}/${BACKUP_S3_PREFIX}/$(date +%Y/%m/%d)/${BACKUP_FILENAME}"
  
  log "Uploading backup to S3: $s3_path"
  
  if [[ "$DRY_RUN" != "true" ]]; then
    # Upload backup file
    if ! aws s3 cp "${sql_file}.gz" "$s3_path" \
      --profile "$AWS_PROFILE" \
      --sse AES256 \
      --storage-class STANDARD_IA; then
      error "S3 upload failed"
    fi
    
    # Upload checksum
    echo "$checksum" | aws s3 cp - "${s3_path}.sha256" \
      --profile "$AWS_PROFILE" \
      --sse AES256
    
    log "✓ Backup uploaded to: $s3_path"
  else
    log "[DRY-RUN] Would upload to: $s3_path"
  fi
}

upload_to_gcs() {
  local sql_file="$1"
  local checksum="$2"
  local file_size="$3"
  
  if [[ -z "$BACKUP_GCS_BUCKET" ]]; then
    error "BACKUP_GCS_BUCKET is required for GCS uploads"
  fi
  
  local gcs_path="gs://${BACKUP_GCS_BUCKET}/trustlink-db-backups/$(date +%Y/%m/%d)/${BACKUP_FILENAME}"
  
  log "Uploading backup to GCS: $gcs_path"
  
  if [[ "$DRY_RUN" != "true" ]]; then
    # Upload backup file
    if ! gsutil -m cp "${sql_file}.gz" "$gcs_path"; then
      error "GCS upload failed"
    fi
    
    # Upload checksum
    echo "$checksum" | gsutil cp - "${gcs_path}.sha256"
    
    log "✓ Backup uploaded to: $gcs_path"
  else
    log "[DRY-RUN] Would upload to: $gcs_path"
  fi
}

cleanup_old_backups() {
  log "Cleaning up backups older than $BACKUP_RETENTION_DAYS days..."
  
  case "$BACKUP_STORAGE" in
    local)
      if [[ "$DRY_RUN" != "true" ]]; then
        find "$BACKUP_PATH" -name "trustlink_db_*.sql.gz" -mtime "+$BACKUP_RETENTION_DAYS" -delete
        log "✓ Old backups cleaned up"
      else
        find "$BACKUP_PATH" -name "trustlink_db_*.sql.gz" -mtime "+$BACKUP_RETENTION_DAYS" | while read f; do
          log "[DRY-RUN] Would delete: $f"
        done
      fi
      ;;
    s3)
      if [[ "$DRY_RUN" != "true" ]]; then
        aws s3 rm "s3://${BACKUP_S3_BUCKET}/${BACKUP_S3_PREFIX}/" \
          --recursive \
          --profile "$AWS_PROFILE" \
          --exclude "*" \
          --include "trustlink_db_*.sql.gz" \
          --older-than-days "$BACKUP_RETENTION_DAYS"
        log "✓ Old S3 backups cleaned up"
      else
        log "[DRY-RUN] Would clean old S3 backups older than $BACKUP_RETENTION_DAYS days"
      fi
      ;;
    gcs)
      log "⚠ GCS cleanup not yet implemented; manage retention via GCS lifecycle policies"
      ;;
  esac
}

verify_backup() {
  local sql_file="$1"
  local checksum="$2"
  
  log "Verifying backup integrity..."
  
  local actual_checksum
  actual_checksum=$(sha256sum "${sql_file}.gz" | awk '{print $1}')
  
  if [[ "$actual_checksum" != "$checksum" ]]; then
    error "Checksum mismatch: expected $checksum, got $actual_checksum"
  fi
  
  # Try to list tables in backup (verify it's valid SQL)
  if ! gunzip -c "${sql_file}.gz" | head -100 | grep -q "CREATE TABLE\|INSERT INTO"; then
    error "Backup file doesn't appear to contain valid PostgreSQL data"
  fi
  
  log "✓ Backup verified"
}

# ───────────────────────────────────────────────────────────────────────────────
# Main
# ───────────────────────────────────────────────────────────────────────────────

main() {
  log "TrustLink Database Backup Script"
  log "Storage: $BACKUP_STORAGE | Retention: $BACKUP_RETENTION_DAYS days | DRY_RUN: $DRY_RUN"
  
  validate_database_connection
  
  local sql_file checksum file_size
  read -r sql_file checksum file_size < <(backup_database)
  
  verify_backup "$sql_file" "$checksum"
  
  case "$BACKUP_STORAGE" in
    local)
      upload_to_local "$sql_file" "$checksum" "$file_size"
      ;;
    s3)
      upload_to_s3 "$sql_file" "$checksum" "$file_size"
      ;;
    gcs)
      upload_to_gcs "$sql_file" "$checksum" "$file_size"
      ;;
    *)
      error "Unknown BACKUP_STORAGE: $BACKUP_STORAGE"
      ;;
  esac
  
  cleanup_old_backups
  
  log "✓ Backup completed successfully"
}

main "$@"
