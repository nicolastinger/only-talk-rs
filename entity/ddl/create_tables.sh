#!/bin/bash

# ========================================
# 数据库表一键创建脚本 (Linux/Mac)
# ========================================
# 使用说明：
# 1. 修改下面的数据库连接信息
# 2. 给脚本添加执行权限：chmod +x create_tables.sh
# 3. 执行脚本：./create_tables.sh
# ========================================

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 数据库连接配置
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-your_database_name}"
DB_USER="${DB_USER:-postgres}"

# DDL 目录
DDL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# 日志文件
LOG_FILE="$DDL_DIR/create_tables_$(date +%Y%m%d_%H%M%S).log"

# SQL 文件执行顺序
SQL_FILES=(
    "sequences.sql"
    "basic_user.sql"
    "basic_user_salt.sql"
    "user_info.sql"
    "user_cache.sql"
    "user_login_log.sql"
    "file_upload_record.sql"
    "biz_record.sql"
    "chat_biz_record.sql"
    "private_biz_record.sql"
    "friend_link.sql"
    "friend_list.sql"
    "friend_request_info.sql"
    "chat_list_link.sql"
    "chat_message_record.sql"
    "chat_message_record_fail.sql"
    "chat_message_record_read.sql"
    "system_notification.sql"
)

# 打印带颜色的消息
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_FILE"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
}

print_separator() {
    echo "========================================" | tee -a "$LOG_FILE"
}

# 检查 psql 是否安装
check_psql() {
    if ! command -v psql &> /dev/null; then
        print_error "psql 未安装，请先安装 PostgreSQL 客户端"
        exit 1
    fi
    print_success "psql 已安装"
}

# 检查数据库连接
check_connection() {
    print_info "检查数据库连接..."
    if ! PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT 1;" &> /dev/null; then
        print_error "无法连接到数据库"
        print_error "请检查以下配置："
        print_error "  - 数据库主机: $DB_HOST:$DB_PORT"
        print_error "  - 数据库名称: $DB_NAME"
        print_error "  - 用户名: $DB_USER"
        print_error "  - 密码: (请设置环境变量 DB_PASSWORD)"
        exit 1
    fi
    print_success "数据库连接成功"
}

# 检查 SQL 文件是否存在
check_sql_files() {
    print_info "检查 SQL 文件..."
    missing_files=()
    
    for file in "${SQL_FILES[@]}"; do
        file_path="$DDL_DIR/$file"
        if [ ! -f "$file_path" ]; then
            missing_files+=("$file")
        fi
    done
    
    if [ ${#missing_files[@]} -gt 0 ]; then
        print_error "以下 SQL 文件不存在："
        for file in "${missing_files[@]}"; do
            print_error "  - $file"
        done
        exit 1
    fi
    print_success "所有 SQL 文件检查通过"
}

# 执行 SQL 文件
execute_sql_file() {
    local file=$1
    local file_path="$DDL_DIR/$file"
    
    print_info "正在执行: $file"
    
    if PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f "$file_path" >> "$LOG_FILE" 2>&1; then
        print_success "✓ $file 执行成功"
        return 0
    else
        print_error "✗ $file 执行失败"
        print_error "请查看日志文件: $LOG_FILE"
        return 1
    fi
}

# 验证表创建
verify_tables() {
    print_info "验证表创建..."
    
    local expected_tables=(
        "basic_user"
        "basic_user_salt"
        "user_info"
        "user_cache"
        "user_login_log"
        "file_upload_record"
        "biz_record"
        "chat_biz_record"
        "private_biz_record"
        "friend_link"
        "friend_list"
        "friend_request_info"
        "chat_list_link"
        "chat_message_record"
        "chat_message_record_fail"
        "chat_message_record_read"
        "system_notification"
    )
    
    local missing_tables=()
    
    for table in "${expected_tables[@]}"; do
        if ! PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "\dt $table" &> /dev/null; then
            missing_tables+=("$table")
        fi
    done
    
    if [ ${#missing_tables[@]} -gt 0 ]; then
        print_warning "以下表未创建成功："
        for table in "${missing_tables[@]}"; do
            print_warning "  - $table"
        done
        return 1
    fi
    
    print_success "所有表创建验证通过"
    return 0
}

# 显示数据库信息
show_database_info() {
    print_separator
    print_info "数据库信息："
    print_info "  主机: $DB_HOST:$DB_PORT"
    print_info "  数据库: $DB_NAME"
    print_info "  用户: $DB_USER"
    print_info "  日志文件: $LOG_FILE"
    print_separator
}

# 主函数
main() {
    print_separator
    print_info "数据库表一键创建脚本"
    print_separator
    
    # 显示数据库信息
    show_database_info
    
    # 检查环境
    check_psql
    check_connection
    check_sql_files
    
    print_separator
    print_info "开始创建表..."
    print_separator
    
    # 记录开始时间
    start_time=$(date +%s)
    
    # 执行 SQL 文件
    success_count=0
    failed_files=()
    
    for file in "${SQL_FILES[@]}"; do
        if execute_sql_file "$file"; then
            ((success_count++))
        else
            failed_files+=("$file")
        fi
    done
    
    # 记录结束时间
    end_time=$(date +%s)
    duration=$((end_time - start_time))
    
    print_separator
    print_info "执行结果："
    print_info "  成功: $success_count / ${#SQL_FILES[@]}"
    print_info "  失败: ${#failed_files[@]} / ${#SQL_FILES[@]}"
    print_info "  耗时: ${duration} 秒"
    print_separator
    
    # 如果有失败的文件，显示详细信息
    if [ ${#failed_files[@]} -gt 0 ]; then
        print_error "以下文件执行失败："
        for file in "${failed_files[@]}"; do
            print_error "  - $file"
        done
        print_error "请查看日志文件: $LOG_FILE"
        exit 1
    fi
    
    # 验证表创建
    if verify_tables; then
        print_separator
        print_success "所有表创建成功！"
        print_separator
        
        # 显示创建的表
        print_info "已创建的表："
        PGPASSWORD="$DB_PASSWORD" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "\dt" | tee -a "$LOG_FILE"
        
        print_separator
        print_success "脚本执行完成！"
        print_info "日志文件: $LOG_FILE"
        print_separator
        exit 0
    else
        print_error "表创建验证失败"
        exit 1
    fi
}

# 执行主函数
main
