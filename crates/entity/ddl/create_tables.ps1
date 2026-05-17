# ========================================
# 数据库表一键创建脚本 (Windows PowerShell)
# ========================================
# 使用说明：
# 1. 修改下面的数据库连接信息
# 2. 以管理员身份运行 PowerShell
# 3. 执行脚本：.\create_tables.ps1
# ========================================

# 数据库连接配置
$DB_HOST = if ($env:DB_HOST) { $env:DB_HOST } else { "localhost" }
$DB_PORT = if ($env:DB_PORT) { $env:DB_PORT } else { "5432" }
$DB_NAME = if ($env:DB_NAME) { $env:DB_NAME } else { "your_database_name" }
$DB_USER = if ($env:DB_USER) { $env:DB_USER } else { "postgres" }

# DDL 目录
$DDL_DIR = Split-Path -Parent $MyInvocation.MyCommand.Path

# 日志文件
$LOG_FILE = Join-Path $DDL_DIR ("create_tables_{0:yyyyMMdd_HHmmss}.log" -f (Get-Date))

# SQL 文件执行顺序
$SQL_FILES = @(
    "sequences.sql",
    "basic_user.sql",
    "basic_user_salt.sql",
    "user_info.sql",
    "user_cache.sql",
    "user_login_log.sql",
    "file_upload_record.sql",
    "biz_record.sql",
    "chat_biz_record.sql",
    "private_biz_record.sql",
    "friend_link.sql",
    "friend_list.sql",
    "friend_request_info.sql",
    "chat_list_link.sql",
    "chat_message_record.sql",
    "chat_message_record_fail.sql",
    "chat_message_record_read.sql",
    "system_notification.sql"
)

# 打印带颜色的消息
function Write-Info {
    param([string]$Message)
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logMessage = "[$timestamp] [INFO] $Message"
    Write-Host $logMessage -ForegroundColor Cyan
    Add-Content -Path $LOG_FILE -Value $logMessage
}

function Write-Success {
    param([string]$Message)
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logMessage = "[$timestamp] [SUCCESS] $Message"
    Write-Host $logMessage -ForegroundColor Green
    Add-Content -Path $LOG_FILE -Value $logMessage
}

function Write-Warning {
    param([string]$Message)
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logMessage = "[$timestamp] [WARNING] $Message"
    Write-Host $logMessage -ForegroundColor Yellow
    Add-Content -Path $LOG_FILE -Value $logMessage
}

function Write-Error {
    param([string]$Message)
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logMessage = "[$timestamp] [ERROR] $Message"
    Write-Host $logMessage -ForegroundColor Red
    Add-Content -Path $LOG_FILE -Value $logMessage
}

function Write-Separator {
    $separator = "========================================"
    Write-Host $separator
    Add-Content -Path $LOG_FILE -Value $separator
}

# 检查 psql 是否安装
function Test-Psql {
    Write-Info "检查 psql 是否安装..."
    
    try {
        $null = Get-Command psql -ErrorAction Stop
        Write-Success "psql 已安装"
        return $true
    }
    catch {
        Write-Error "psql 未安装，请先安装 PostgreSQL 客户端"
        Write-Error "下载地址: https://www.postgresql.org/download/windows/"
        return $false
    }
}

# 检查数据库连接
function Test-DatabaseConnection {
    Write-Info "检查数据库连接..."
    
    $env:PGPASSWORD = $env:DB_PASSWORD
    
    try {
        $result = psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c "SELECT 1;" 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Success "数据库连接成功"
            return $true
        }
        else {
            Write-Error "无法连接到数据库"
            Write-Error "请检查以下配置："
            Write-Error "  - 数据库主机: $DB_HOST:$DB_PORT"
            Write-Error "  - 数据库名称: $DB_NAME"
            Write-Error "  - 用户名: $DB_USER"
            Write-Error "  - 密码: (请设置环境变量 DB_PASSWORD)"
            return $false
        }
    }
    catch {
        Write-Error "数据库连接异常: $_"
        return $false
    }
}

# 检查 SQL 文件是否存在
function Test-SqlFiles {
    Write-Info "检查 SQL 文件..."
    
    $missingFiles = @()
    
    foreach ($file in $SQL_FILES) {
        $filePath = Join-Path $DDL_DIR $file
        if (-not (Test-Path $filePath)) {
            $missingFiles += $file
        }
    }
    
    if ($missingFiles.Count -gt 0) {
        Write-Error "以下 SQL 文件不存在："
        foreach ($file in $missingFiles) {
            Write-Error "  - $file"
        }
        return $false
    }
    
    Write-Success "所有 SQL 文件检查通过"
    return $true
}

# 执行 SQL 文件
function Invoke-SqlFile {
    param([string]$File)
    
    $filePath = Join-Path $DDL_DIR $File
    
    Write-Info "正在执行: $File"
    
    $env:PGPASSWORD = $env:DB_PASSWORD
    
    try {
        $output = psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -f $filePath 2>&1
        $output | Out-File -FilePath $LOG_FILE -Append
        
        if ($LASTEXITCODE -eq 0) {
            Write-Success "✓ $File 执行成功"
            return $true
        }
        else {
            Write-Error "✗ $File 执行失败"
            Write-Error "请查看日志文件: $LOG_FILE"
            return $false
        }
    }
    catch {
        Write-Error "执行 $File 时发生异常: $_"
        return $false
    }
}

# 验证表创建
function Test-TablesCreated {
    Write-Info "验证表创建..."
    
    $expectedTables = @(
        "basic_user",
        "basic_user_salt",
        "user_info",
        "user_cache",
        "user_login_log",
        "file_upload_record",
        "biz_record",
        "chat_biz_record",
        "private_biz_record",
        "friend_link",
        "friend_list",
        "friend_request_info",
        "chat_list_link",
        "chat_message_record",
        "chat_message_record_fail",
        "chat_message_record_read",
        "system_notification"
    )
    
    $missingTables = @()
    
    $env:PGPASSWORD = $env:DB_PASSWORD
    
    foreach ($table in $expectedTables) {
        $result = psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c "\dt $table" 2>&1
        if ($LASTEXITCODE -ne 0) {
            $missingTables += $table
        }
    }
    
    if ($missingTables.Count -gt 0) {
        Write-Warning "以下表未创建成功："
        foreach ($table in $missingTables) {
            Write-Warning "  - $table"
        }
        return $false
    }
    
    Write-Success "所有表创建验证通过"
    return $true
}

# 显示数据库信息
function Show-DatabaseInfo {
    Write-Separator
    Write-Info "数据库信息："
    Write-Info "  主机: $DB_HOST:$DB_PORT"
    Write-Info "  数据库: $DB_NAME"
    Write-Info "  用户: $DB_USER"
    Write-Info "  日志文件: $LOG_FILE"
    Write-Separator
}

# 主函数
function Main {
    Write-Separator
    Write-Info "数据库表一键创建脚本"
    Write-Separator
    
    # 显示数据库信息
    Show-DatabaseInfo
    
    # 检查环境
    if (-not (Test-Psql)) {
        exit 1
    }
    
    if (-not (Test-DatabaseConnection)) {
        exit 1
    }
    
    if (-not (Test-SqlFiles)) {
        exit 1
    }
    
    Write-Separator
    Write-Info "开始创建表..."
    Write-Separator
    
    # 记录开始时间
    $startTime = Get-Date
    
    # 执行 SQL 文件
    $successCount = 0
    $failedFiles = @()
    
    foreach ($file in $SQL_FILES) {
        if (Invoke-SqlFile -File $file) {
            $successCount++
        }
        else {
            $failedFiles += $file
        }
    }
    
    # 记录结束时间
    $endTime = Get-Date
    $duration = ($endTime - $startTime).TotalSeconds
    
    Write-Separator
    Write-Info "执行结果："
    Write-Info "  成功: $successCount / $($SQL_FILES.Count)"
    Write-Info "  失败: $($failedFiles.Count) / $($SQL_FILES.Count)"
    Write-Info "  耗时: $([math]::Round($duration, 2)) 秒"
    Write-Separator
    
    # 如果有失败的文件，显示详细信息
    if ($failedFiles.Count -gt 0) {
        Write-Error "以下文件执行失败："
        foreach ($file in $failedFiles) {
            Write-Error "  - $file"
        }
        Write-Error "请查看日志文件: $LOG_FILE"
        exit 1
    }
    
    # 验证表创建
    if (Test-TablesCreated) {
        Write-Separator
        Write-Success "所有表创建成功！"
        Write-Separator
        
        # 显示创建的表
        Write-Info "已创建的表："
        $env:PGPASSWORD = $env:DB_PASSWORD
        psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c "\dt" | Tee-Object -FilePath $LOG_FILE -Append
        
        Write-Separator
        Write-Success "脚本执行完成！"
        Write-Info "日志文件: $LOG_FILE"
        Write-Separator
        exit 0
    }
    else {
        Write-Error "表创建验证失败"
        exit 1
    }
}

# 执行主函数
Main
