#!/bin/bash

# S3 Service 测试脚本 (Bash)

echo "========================================"
echo "  S3 Service 测试套件"
echo "========================================"
echo ""

# 切换到项目根目录
cd "$(dirname "$0")/.."

echo "1. 运行单元测试（不需要 MinIO 服务）"
echo "----------------------------------------"
cargo test -p s3_service --lib

if [ $? -eq 0 ]; then
    echo ""
    echo "✓ 单元测试通过"
else
    echo ""
    echo "✗ 单元测试失败"
    exit 1
fi

echo ""
echo "2. 运行集成测试（需要 MinIO 服务运行）"
echo "----------------------------------------"
echo "请确保 MinIO 服务已启动："
echo "  端点: http://101.33.75.40:19000"
echo "  用户: minioadmin"
echo ""

read -p "是否继续运行集成测试? (y/n) " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    cargo test -p s3_service --test minio_integration_test
    
    if [ $? -eq 0 ]; then
        echo ""
        echo "✓ 集成测试通过"
    else
        echo ""
        echo "✗ 集成测试失败"
        exit 1
    fi
else
    echo "跳过集成测试"
fi

echo ""
echo "========================================"
echo "  测试完成"
echo "========================================"