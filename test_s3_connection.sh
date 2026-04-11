#!/bin/bash

# S3 连接诊断测试脚本

echo "====== S3 连接诊断工具 ======"
echo ""
echo "此脚本将诊断 S3/MinIO 连接问题"
echo ""

# 运行诊断测试
echo "正在运行诊断测试..."
cargo test --package s3_service --lib tests::diagnose_s3_connection --features "integration-test" -- --nocapture

echo ""
echo "====== 诊断完成 ======"