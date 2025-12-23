use actix_multipart::Multipart;
use actix_web::{web, Responder, Result};
use futures_util::TryStreamExt as _;
use std::path::Path;
use std::fs;
use serde_json::json;
use crate::common::{ApiResponse, Meta};
use std::io::Write;

// 上传文件信息结构体
#[derive(serde::Serialize)]
struct UploadFileInfo {
    filename: String,
    size: u64,
    path: String,
}

// 单个文件上传处理函数
pub async fn upload_single_file(mut payload: Multipart) -> Result<impl Responder> {
    // 创建上传目录（如果不存在）
    let upload_dir = Path::new("uploads");
    if !upload_dir.exists() {
        fs::create_dir_all(upload_dir).unwrap();
    }

    // 处理multipart流
    while let Some(mut field) = payload.try_next().await? {
        let content_disposition = field.content_disposition();
        let filename = content_disposition
            .get_filename()
            .unwrap_or("unknown_file")
            .to_string();

        // 生成唯一文件名（使用时间戳）
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let file_extension = Path::new(&filename)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");
        let unique_filename = if file_extension.is_empty() {
            format!("{}_{}", timestamp, filename)
        } else {
            format!("{}_{}.{}", timestamp, filename.replace(&format!(".{}", file_extension), ""), file_extension)
        };

        let filepath = upload_dir.join(&unique_filename);
        let mut file = std::fs::File::create(filepath)?;

        // 读取文件数据并写入磁盘
        let mut total_size = 0;
        while let Some(chunk) = field.try_next().await? {
            total_size += chunk.len() as u64;
            file.write_all(&chunk)?;
        }

        let result = UploadFileInfo {
            filename,
            size: total_size,
            path: format!("/uploads/{}", unique_filename),
        };
        
        // 如果只有一个文件，返回单个文件信息
        return Ok(ApiResponse::success(result));
    }

    Ok(ApiResponse::error(400, "No files uploaded"))
}

// 批量文件上传处理函数
pub async fn upload_batch_files(mut payload: Multipart) -> Result<impl Responder> {
    let mut upload_results = Vec::new();
    
    // 创建上传目录（如果不存在）
    let upload_dir = Path::new("uploads");
    if !upload_dir.exists() {
        fs::create_dir_all(upload_dir).unwrap();
    }

    // 处理multipart流
    while let Some(mut field) = payload.try_next().await? {
        let content_disposition = field.content_disposition();
        let filename = content_disposition
            .get_filename()
            .unwrap_or("unknown_file")
            .to_string();

        // 生成唯一文件名（使用时间戳）
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let file_extension = Path::new(&filename)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");
        let unique_filename = if file_extension.is_empty() {
            format!("{}_{}", timestamp, filename)
        } else {
            format!("{}_{}.{}", timestamp, filename.replace(&format!(".{}", file_extension), ""), file_extension)
        };

        let filepath = upload_dir.join(&unique_filename);
        let mut file = std::fs::File::create(filepath)?;

        // 读取文件数据并写入磁盘
        let mut total_size = 0;
        while let Some(chunk) = field.try_next().await? {
            total_size += chunk.len() as u64;
            file.write_all(&chunk)?;
        }

        upload_results.push(UploadFileInfo {
            filename,
            size: total_size,
            path: format!("/uploads/{}", unique_filename),
        });
    }

    if upload_results.is_empty() {
        return Ok(ApiResponse::error(400, "No files uploaded"));
    }

    let meta = Meta {
        total_items: upload_results.len() as u64,
        total_pages: 0,
        current_page: 0,
        page_size: 0,
    };
    
    Ok(ApiResponse::success_with_meta(upload_results, meta))
}

// 获取上传文件列表
pub async fn list_uploaded_files() -> Result<impl Responder> {
    let upload_dir = Path::new("uploads");
    
    if !upload_dir.exists() {
        return Ok(ApiResponse::success(json!([])));
    }

    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(upload_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    if let Some(filename) = entry.file_name().to_str() {
                        let metadata = entry.metadata().unwrap_or_else(|_| std::fs::metadata(entry.path()).unwrap());
                        files.push(json!({
                            "filename": filename,
                            "size": metadata.len(),
                            "path": format!("/uploads/{}", filename),
                            "created": metadata.created().map(|t| format!("{:?}", t)).unwrap_or_default()
                        }));
                    }
                }
            }
        }
    }

    Ok(ApiResponse::success(serde_json::Value::Array(files)))
}

// 配置文件上传路由
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/files")
            .route("/upload", web::post().to(upload_single_file))
            .route("/upload/batch", web::post().to(upload_batch_files))
            .route("/list", web::get().to(list_uploaded_files))
    );
}