@echo off
REM ========================================
REM 数据库表一键创建脚本 (Windows CMD)
REM ========================================
REM 使用说明：
REM 1. 修改下面的数据库连接信息
REM 2. 双击执行或在 CMD 中运行
REM 3. 执行脚本：create_tables.bat
REM ========================================

setlocal enabledelayedexpansion

REM 数据库连接配置
if "%DB_HOST%"=="" set DB_HOST=localhost
if "%DB_PORT%"=="" set DB_PORT=5432
if "%DB_NAME%"=="" set DB_NAME=your_database_name
if "%DB_USER%"=="" set DB_USER=postgres

REM DDL 目录
set DDL_DIR=%~dp0

REM 日志文件
set LOG_FILE=%DDL_DIR%create_tables_%date:~0,4%%date:~5,2%%date:~8,2%_%time:~0,2%%time:~3,2%%time:~6,2%.log
REM 替换日志文件名中的空格
set LOG_FILE=%LOG_FILE: =0%

REM SQL 文件执行顺序
set SQL_FILES[0]=sequences.sql
set SQL_FILES[1]=basic_user.sql
set SQL_FILES[2]=basic_user_salt.sql
set SQL_FILES[3]=user_info.sql
set SQL_FILES[4]=user_cache.sql
set SQL_FILES[5]=user_login_log.sql
set SQL_FILES[6]=file_upload_record.sql
set SQL_FILES[7]=biz_record.sql
set SQL_FILES[8]=chat_biz_record.sql
set SQL_FILES[9]=private_biz_record.sql
set SQL_FILES[10]=friend_link.sql
set SQL_FILES[11]=friend_list.sql
set SQL_FILES[12]=friend_request_info.sql
set SQL_FILES[13]=chat_list_link.sql
set SQL_FILES[14]=chat_message_record.sql
set SQL_FILES[15]=chat_message_record_fail.sql
set SQL_FILES[16]=chat_message_record_read.sql
set SQL_FILES[17]=system_notification.sql

echo ========================================
echo 数据库表一键创建脚本
echo ========================================
echo.

echo 数据库信息：
echo   主机: %DB_HOST%:%DB_PORT%
echo   数据库: %DB_NAME%
echo   用户: %DB_USER%
echo   日志文件: %LOG_FILE%
echo ========================================
echo.

REM 检查 psql 是否安装
echo [INFO] 检查 psql 是否安装...
where psql >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] psql 未安装，请先安装 PostgreSQL 客户端
    echo [ERROR] 下载地址: https://www.postgresql.org/download/windows/
    pause
    exit /b 1
)
echo [SUCCESS] psql 已安装
echo.

REM 检查数据库连接
echo [INFO] 检查数据库连接...
set PGPASSWORD=%DB_PASSWORD%
psql -h %DB_HOST% -p %DB_PORT% -U %DB_USER% -d %DB_NAME% -c "SELECT 1;" >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] 无法连接到数据库
    echo [ERROR] 请检查以下配置：
    echo [ERROR]   - 数据库主机: %DB_HOST%:%DB_PORT%
    echo [ERROR]   - 数据库名称: %DB_NAME%
    echo [ERROR]   - 用户名: %DB_USER%
    echo [ERROR]   - 密码: (请设置环境变量 DB_PASSWORD)
    pause
    exit /b 1
)
echo [SUCCESS] 数据库连接成功
echo.

REM 检查 SQL 文件是否存在
echo [INFO] 检查 SQL 文件...
set missing_files=0
for /L %%i in (0,1,17) do (
    set file=!SQL_FILES[%%i]!
    if not exist "%DDL_DIR%!file!" (
        echo [ERROR] 文件不存在: !file!
        set /a missing_files+=1
    )
)
if %missing_files% gtr 0 (
    echo [ERROR] 有 %missing_files% 个 SQL 文件不存在
    pause
    exit /b 1
)
echo [SUCCESS] 所有 SQL 文件检查通过
echo.

echo ========================================
echo [INFO] 开始创建表...
echo ========================================
echo.

REM 记录开始时间
set start_time=%time%

REM 执行 SQL 文件
set success_count=0
set failed_count=0

for /L %%i in (0,1,17) do (
    set file=!SQL_FILES[%%i]!
    set file_path=%DDL_DIR%!file!
    
    echo [INFO] 正在执行: !file!
    
    psql -h %DB_HOST% -p %DB_PORT% -U %DB_USER% -d %DB_NAME% -f "!file_path!" >> "%LOG_FILE%" 2>&1
    
    if !errorlevel! equ 0 (
        echo [SUCCESS] ✓ !file! 执行成功
        set /a success_count+=1
    ) else (
        echo [ERROR] ✗ !file! 执行失败
        echo [ERROR] 请查看日志文件: %LOG_FILE%
        set /a failed_count+=1
    )
    echo.
)

REM 记录结束时间
set end_time=%time%

echo ========================================
echo [INFO] 执行结果：
echo   成功: %success_count% / 18
echo   失败: %failed_count% / 18
echo ========================================
echo.

REM 如果有失败的文件，显示详细信息
if %failed_count% gtr 0 (
    echo [ERROR] 有 %failed_count% 个文件执行失败
    echo [ERROR] 请查看日志文件: %LOG_FILE%
    pause
    exit /b 1
)

REM 验证表创建
echo [INFO] 验证表创建...
echo.

REM 显示创建的表
echo [INFO] 已创建的表：
psql -h %DB_HOST% -p %DB_PORT% -U %DB_USER% -d %DB_NAME% -c "\dt"
echo.

echo ========================================
echo [SUCCESS] 所有表创建成功！
echo ========================================
echo [INFO] 日志文件: %LOG_FILE%
echo ========================================
echo.

pause
