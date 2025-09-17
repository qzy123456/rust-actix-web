use std::fs::{self, File, OpenOptions}; 
use std::io::{self, BufWriter, Write}; 
use std::path::Path; 
use chrono::{DateTime, Local}; 
use serde::{Serialize, Deserialize}; 
use serde_json; 
use std::sync::{Arc, Mutex}; 
use std::fmt; 

// 日志级别枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    TRACE,
    DEBUG,
    INFO,
    WARNING,
    ERROR,
    FATAL,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::TRACE => write!(f, "TRACE"),
            LogLevel::DEBUG => write!(f, "DEBUG"),
            LogLevel::INFO => write!(f, "INFO"),
            LogLevel::WARNING => write!(f, "WARNING"),
            LogLevel::ERROR => write!(f, "ERROR"),
            LogLevel::FATAL => write!(f, "FATAL"),
        }
    }
}

// 日志条目结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    timestamp: String,         // 时间戳
    level: String,             // 日志级别
    message: String,           // 日志消息
    module: Option<String>,    // 模块名称
    file: Option<String>,      // 文件路径
    line: Option<u32>,         // 行号
    additional_data: Option<serde_json::Value>, // 附加数据
}

// JSON日志器配置
pub struct JsonLoggerConfig {
    pub log_dir: String,          // 日志目录
    pub max_file_size_mb: u64,    // 最大文件大小(MB)
    pub min_level: LogLevel,      // 最小日志级别
}

impl Default for JsonLoggerConfig {
    fn default() -> Self {
        Self {
            log_dir: String::from("logs"),
            max_file_size_mb: 10,
            min_level: LogLevel::INFO,
        }
    }
}

// JSON日志器实现
pub struct JsonLogger {
    config: JsonLoggerConfig,
    current_date: String,        // 当前日期(YYYY-MM-DD)
    current_file_index: u32,     // 当前文件索引
    file_writer: Arc<Mutex<Option<BufWriter<File>>>>, // 文件写入器
}

impl JsonLogger {
    // 创建新的JSON日志器
    pub fn new(config: JsonLoggerConfig) -> Result<Self, io::Error> {
        // 确保日志目录存在
        fs::create_dir_all(&config.log_dir)?;
        
        let now = Local::now();
        let current_date = now.format("%Y-%m-%d").to_string();
        
        let mut logger = Self {
            config,
            current_date,
            current_file_index: 0,
            file_writer: Arc::new(Mutex::new(None)),
        };
        
        // 初始化文件写入器
        logger.init_log_file()?;
        
        Ok(logger)
    }
    
    // 初始化日志文件
    fn init_log_file(&mut self) -> Result<(), io::Error> {
        // 检查是否需要更新日期
        let now = Local::now();
        let today = now.format("%Y-%m-%d").to_string();
        
        // 如果日期变更，重置文件索引
        if today != self.current_date {
            self.current_date = today;
            self.current_file_index = 0;
        }
        
        // 查找当前日期的最大文件索引
        let mut max_index = 0;

        if let Ok(entries) = fs::read_dir(&self.config.log_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.starts_with(&format!("{}-{}-", self.current_date, "app")) {
                            if let Some(index_str) = file_name.strip_prefix(&format!("{}_{}_", self.current_date, "app")).and_then(|s| s.strip_suffix(".log")) {
                                if let Ok(index) = index_str.parse::<u32>() {
                                    if index > max_index {
                                        max_index = index;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        self.current_file_index = max_index;
        
        // 检查当前文件大小，如果超过限制则创建新文件
        let file_path = self.get_log_file_path();
        if Path::new(&file_path).exists() {
            if let Ok(metadata) = fs::metadata(&file_path) {
                let file_size_mb = metadata.len() / (1024 * 1024);
                if file_size_mb >= self.config.max_file_size_mb {
                    self.current_file_index += 1;
                }
            }
        }
        
        // 打开或创建日志文件
        let file_path = self.get_log_file_path();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;
        
        let writer = BufWriter::new(file);
        *self.file_writer.lock().unwrap() = Some(writer);
        
        Ok(())
    }
    
    // 获取日志文件路径
    fn get_log_file_path(&self) -> String {
        Path::new(&self.config.log_dir)
            .join(format!("{}-{}-{}.log", self.current_date, "app", self.current_file_index))
            .to_str()
            .unwrap()
            .to_string()
    }
    
    // 检查并可能分割日志文件
    fn check_and_rotate(&mut self) -> Result<(), io::Error> {
        let file_path = self.get_log_file_path();
        if let Ok(metadata) = fs::metadata(&file_path) {
            let file_size_mb = metadata.len() / (1024 * 1024);
            if file_size_mb >= self.config.max_file_size_mb {
                self.current_file_index += 1;
                self.init_log_file()?;
            }
        }
        
        // 检查日期是否变更
        let now = Local::now();
        let today = now.format("%Y-%m-%d").to_string();
        if today != self.current_date {
            self.init_log_file()?;
        }
        
        Ok(())
    }
    
    // 记录日志
    pub fn log(&mut self, level: LogLevel, message: &str, module: Option<&str>, file: Option<&str>, line: Option<u32>, additional_data: Option<serde_json::Value>) -> Result<(), io::Error> {
        // 检查日志级别
        if level < self.config.min_level {
            return Ok(());
        }
        
        // 检查并可能分割日志文件
        self.check_and_rotate()?;
        
        // 创建日志条目
        let now: DateTime<Local> = Local::now();
        let log_entry = LogEntry {
            timestamp: now.to_rfc3339(),
            level: level.to_string(),
            message: message.to_string(),
            module: module.map(|s| s.to_string()),
            file: file.map(|s| s.to_string()),
            line,
            additional_data,
        };
        
        // 序列化日志条目为JSON
        let json = serde_json::to_string(&log_entry)?;
        
        // 写入日志文件
        if let Some(ref mut writer) = *self.file_writer.lock().unwrap() {
            writeln!(writer, "{}", json)?;
            writer.flush()?;
        }
        
        Ok(())
    }
    
    // 便捷方法：记录TRACE级别的日志
    pub fn trace(&mut self, message: &str) -> Result<(), io::Error> {
        self.log(LogLevel::TRACE, message, None, None, None, None)
    }
    
    // 便捷方法：记录DEBUG级别的日志
    pub fn debug(&mut self, message: &str) -> Result<(), io::Error> {
        self.log(LogLevel::DEBUG, message, None, None, None, None)
    }
    
    // 便捷方法：记录INFO级别的日志
    pub fn info(&mut self, message: &str) -> Result<(), io::Error> {
        self.log(LogLevel::INFO, message, None, None, None, None)
    }
    
    // 便捷方法：记录WARNING级别的日志
    pub fn warning(&mut self, message: &str) -> Result<(), io::Error> {
        self.log(LogLevel::WARNING, message, None, None, None, None)
    }
    
    // 便捷方法：记录ERROR级别的日志
    pub fn error(&mut self, message: &str) -> Result<(), io::Error> {
        self.log(LogLevel::ERROR, message, None, None, None, None)
    }
    
    // 便捷方法：记录FATAL级别的日志
    pub fn fatal(&mut self, message: &str) -> Result<(), io::Error> {
        self.log(LogLevel::FATAL, message, None, None, None, None)
    }
    
    // 带附加信息的日志方法
    pub fn log_with_data(&mut self, level: LogLevel, message: &str, data: serde_json::Value) -> Result<(), io::Error> {
        self.log(level, message, None, None, None, Some(data))
    }
    
    // 记录详细日志，包括模块、文件和行号
    pub fn log_detailed(&mut self, level: LogLevel, message: &str, module: &str, file: &str, line: u32) -> Result<(), io::Error> {
        self.log(level, message, Some(module), Some(file), Some(line), None)
    }
}