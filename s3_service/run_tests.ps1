# S3 Service 测试脚本 (PowerShell)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  S3 Service 测试套件" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 切换到项目根目录
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $projectRoot

Write-Host "1. 运行单元测试（不需要 MinIO 服务）" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray
cargo test -p s3_service --lib

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "✓ 单元测试通过" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "✗ 单元测试失败" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "2. 运行集成测试（需要 MinIO 服务运行）" -ForegroundColor Yellow
Write-Host "----------------------------------------" -ForegroundColor Gray
Write-Host "请确保 MinIO 服务已启动：" -ForegroundColor Gray
Write-Host "  端点: http://101.33.75.40:19000" -ForegroundColor Gray
Write-Host "  用户: minioadmin" -ForegroundColor Gray
Write-Host ""

$continue = Read-Host "是否继续运行集成测试? (y/n)"

if ($continue -eq 'y' -or $continue -eq 'Y') {
    cargo test -p s3_service --test minio_integration_test
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✓ 集成测试通过" -ForegroundColor Green
    } else {
        Write-Host ""
        Write-Host "✗ 集成测试失败" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "跳过集成测试" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  测试完成" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan