use actix_files::NamedFile;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Result, web};
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::PathBuf;

// 媒体文件信息
struct MediaFile {
    path: PathBuf,
    mime_type: String,
}

// 解析文件路径并获取MIME类型
fn get_media_file(path: PathBuf) -> Result<MediaFile, String> {
    if !path.exists() {
        return Err("文件不存在".to_string());
    }

    // 根据扩展名确定MIME类型
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime_type = match extension.as_str() {
        "mp4" | "m4v" | "mov" => "video/mp4".to_string(),
        "mkv" => "video/x-matroska".to_string(),
        "avi" => "video/x-msvideo".to_string(),
        "wmv" => "video/x-ms-wmv".to_string(),
        "mpg" | "mpeg" => "video/mpeg".to_string(),
        "flv" => "video/x-flv".to_string(),
        "webm" => "video/webm".to_string(),
        "ts" => "video/mp2t".to_string(),
        "vob" => "video/mpeg".to_string(),
        "m2ts" => "video/mp2t".to_string(),
        _ => "video/*".to_string(),
    };

    Ok(MediaFile { path, mime_type })
}

// 处理DLNA媒体请求
async fn handle_dlna_media(
    req: HttpRequest,
    file_path: web::Path<String>,
    media_dir: web::Data<String>,
) -> Result<HttpResponse> {
    // 构建完整文件路径
    let base_path = PathBuf::from(&**media_dir);
    let requested_path = PathBuf::from(&*file_path);

    // 防止目录遍历攻击
    let full_path = base_path.join(&requested_path);

    // 规范化路径并确保它在基础目录内
    let canonical_base = match base_path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().body("无法解析基础目录路径"));
        }
    };

    let canonical_full = match full_path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            return Ok(HttpResponse::NotFound().body("文件不存在或无法访问"));
        }
    };

    // 检查请求的文件是否在基础目录内
    if !canonical_full.starts_with(&canonical_base) {
        return Ok(HttpResponse::Forbidden().body("禁止访问基础目录之外的文件"));
    }

    let media_file = match get_media_file(canonical_full) {
        Ok(file) => file,
        Err(e) => {
            return Ok(HttpResponse::NotFound().content_type("text/plain").body(e));
        }
    };

    // 检查范围请求（DLNA需要支持）
    let range_header = req.headers().get("Range");

    if let Some(range) = range_header {
        // 处理范围请求
        let range_str = range.to_str().unwrap_or("");
        let file_size = match std::fs::metadata(&media_file.path) {
            Ok(metadata) => metadata.len(),
            Err(_) => {
                return Ok(HttpResponse::InternalServerError().body("无法获取文件大小"));
            }
        };

        // 解析Range头，格式如: bytes=0-499999
        if let Some(range) = parse_range(range_str, file_size) {
            let mut file = File::open(&media_file.path).unwrap();
            file.seek(SeekFrom::Start(range.start)).unwrap();

            let mut buffer = vec![0; (range.end - range.start + 1) as usize];
            file.read_exact(&mut buffer).unwrap();

            let content_length = buffer.len();
            let content_range = format!("bytes {}-{}/{}", range.start, range.end, file_size);

            return Ok(HttpResponse::PartialContent()
                .insert_header(("Content-Type", media_file.mime_type.clone()))
                .insert_header(("Content-Length", content_length.to_string()))
                .insert_header(("Content-Range", content_range))
                .insert_header(("Accept-Ranges", "bytes"))
                .insert_header(("Cache-Control", "no-cache, must-revalidate"))
                .insert_header(("Pragma", "no-cache"))
                .insert_header(("Expires", "0"))
                // DLNA响应头
                .insert_header(("transferMode.dlna.org", "Streaming"))
                .insert_header((
                    "contentFeatures.dlna.org",
                    "DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000",
                ))
                .body(buffer));
        }
    }

    // 全文件请求
    let file = match NamedFile::open(&media_file.path) {
        Ok(f) => f,
        Err(_) => {
            return Ok(HttpResponse::NotFound().body("文件无法打开"));
        }
    };

    // 1. 先设置文件特有的属性（如 Content-Type），并转换成 HttpResponse
    let mut response = file.into_response(&req);

    // 2. 获取 Response 的 HeaderMap 可变引用，手动插入 Header
    let headers = response.headers_mut();

    // 插入标准头 (注意：HeaderName 必须是小写，actix-web/http 库要求)
    headers.insert(
        HeaderName::from_static("accept-ranges"),
        HeaderValue::from_static("bytes"),
    );
    headers.insert(
        HeaderName::from_static("cache-control"),
        HeaderValue::from_static("no-cache, must-revalidate"),
    );
    headers.insert(
        HeaderName::from_static("pragma"),
        HeaderValue::from_static("no-cache"),
    );
    headers.insert(
        HeaderName::from_static("expires"),
        HeaderValue::from_static("0"),
    );

    // 插入 DLNA 特有头
    // 注意：transferMode.dlna.org 在 HTTP 规范中应视为不区分大小写，但在 Rust http 库中 Key 需全小写
    headers.insert(
        HeaderName::from_static("transfermode.dlna.org"),
        HeaderValue::from_static("Streaming"),
    );

    // 对于包含特殊字符（如分号、等号）的值，使用 from_str 或 from_bytes
    if let Ok(val) = HeaderValue::from_str(
        "DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000",
    ) {
        headers.insert(HeaderName::from_static("contentfeatures.dlna.org"), val);
    }

    Ok(response)
}

// 解析Range头
struct Range {
    start: u64,
    end: u64,
}

fn parse_range(range_str: &str, file_size: u64) -> Option<Range> {
    if !range_str.starts_with("bytes=") {
        return None;
    }

    let range_part = &range_str[6..]; // 去掉 "bytes="

    if let Some((start_str, end_str)) = range_part.split_once('-') {
        let start = if start_str.is_empty() {
            // 如: bytes=-500 表示最后500字节
            let len = end_str.parse::<u64>().unwrap_or(0);
            if len > file_size { 0 } else { file_size - len }
        } else {
            start_str.parse::<u64>().unwrap_or(0)
        };

        let end = if end_str.is_empty() {
            file_size - 1
        } else {
            let end_val = end_str.parse::<u64>().unwrap_or(file_size - 1);
            if end_val >= file_size {
                file_size - 1
            } else {
                end_val
            }
        };

        if start <= end && end < file_size {
            return Some(Range { start, end });
        }
    }

    None
}

// 启动媒体服务器
pub async fn start_media_server(host: &str, port: u16, media_dir: &str) -> std::io::Result<()> {
    let media_dir = media_dir.to_string();

    println!("Media server starting on http://{}:{}", host, port);
    println!("Serving files from: {}", media_dir);

    HttpServer::new(move || {
        let media_dir = media_dir.clone();
        App::new()
            .app_data(web::Data::new(media_dir))
            .service(web::resource("/media/{file_path:.*}").route(web::get().to(handle_dlna_media)))
    })
    .bind((host, port))?
    .run()
    .await
}

// 使用示例
#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_media_server() {
        // 启动服务器示例
        // start_media_server("0.0.0.0", 8080, "./videos").await.unwrap();
    }
}
