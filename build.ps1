# TrustLink Build Script
Write-Host "Building TrustLink Smart Contract..." -ForegroundColor Cyan

# Build the contract
Write-Host "`nStep 1: Building contract..." -ForegroundColor Yellow
cargo build --target wasm32-unknown-unknown --release

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Build successful!" -ForegroundColor Green
    
    # Run tests
    Write-Host "`nStep 2: Running tests..." -ForegroundColor Yellow
    cargo test
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ All tests passed!" -ForegroundColor Green
    } else {
        Write-Host "✗ Tests failed" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "✗ Build failed" -ForegroundColor Red
    exit 1
}

Write-Host "`n✓ TrustLink is ready!" -ForegroundColor Green
