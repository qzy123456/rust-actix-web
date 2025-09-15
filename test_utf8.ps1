# 确保输出编码为UTF-8
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()

# 创建临时文件
$tempFile = [System.IO.Path]::GetTempFileName()

# 直接使用UTF-8编码写入文件，避免编码转换问题
$utf8NoBom = New-Object System.Text.UTF8Encoding($False)
[System.IO.File]::WriteAllText($tempFile, '{"phone": "13800138000", "name": "张三", "avatar": 1}', $utf8NoBom)

Write-Host "发送请求..."

# 使用try-catch块来捕获和显示错误
try {
    $response = Invoke-WebRequest -Uri "http://127.0.0.1:8080/users" -Method POST -ContentType "application/json; charset=utf-8" -InFile $tempFile
    Write-Host "响应状态码: $($response.StatusCode)"
    Write-Host "响应内容:" -ForegroundColor Green
    [System.Text.Encoding]::UTF8.GetString([System.Text.Encoding]::Default.GetBytes($response.Content))
} catch {
    Write-Host "请求失败: $($_.Exception.Message)" -ForegroundColor Red
    if ($_.Exception.Response) {
        $errorContent = $_.Exception.Response.GetResponseStream()
        $reader = New-Object System.IO.StreamReader($errorContent)
        $errorText = $reader.ReadToEnd()
        Write-Host "错误响应内容:" -ForegroundColor Red
        [System.Text.Encoding]::UTF8.GetString([System.Text.Encoding]::Default.GetBytes($errorText))
    }
}

# 清理临时文件
Remove-Item $tempFile -Force