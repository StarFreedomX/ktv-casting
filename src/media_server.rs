// 使用示例
use crate::bilibili_parser::get_bilibili_direct_link;
use actix_web::{HttpResponse, get, web};
use futures_util::StreamExt;
use log::info;

#[get("/{url:.*}")]
pub async fn proxy_handler(
    path: web::Path<(String,)>,
    client: web::Data<reqwest::Client>,
) -> Result<HttpResponse, actix_web::Error> {
    let (origin_url,) = path.into_inner();
    info!("Proxying request for URL: {}", origin_url);
    let bv_id = &origin_url[..origin_url.find('-').unwrap_or(origin_url.len())];
    let page: Option<u32> = if let Some(pos) = origin_url.find("-page") {
        origin_url[pos + 5..].parse().ok()
    } else {
        None
    };
    let target_url = get_bilibili_direct_link(&bv_id, page)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let response = client
        .get(&target_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36")
        .header("Referer", "https://www.bilibili.com/") // 加上这个通常更稳
        .send()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let status_u16 = response.status().as_u16();
    let mut client_resp = HttpResponse::build(
        actix_web::http::StatusCode::from_u16(status_u16)
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
    );

    for (name, value) in response.headers().iter() {
        let name_str = name.as_str();
        if name_str != "connection"
            && name_str != "content-encoding"
            && name_str != "transfer-encoding"
        {
            client_resp.insert_header((name_str, value.as_bytes()));
        }
    }

    let body_stream = response
        .bytes_stream()
        .map(|item| item.map_err(|e| std::io::Error::other(e)));

    Ok(client_resp.streaming(body_stream))
}

#[cfg(test)]
mod tests {
    use crate::media_server::proxy_handler;
    use actix_web::{App, HttpServer, web};
    use reqwest::Client;

    #[tokio::test]
    async fn test_https() {
        let client = reqwest::Client::new();

        match client
            .get("https://www.bilibili.com/")
            .header("User-Agent", "Mozilla/5.0 ...")
            .send()
            .await
        {
            Ok(res) => println!("成功连接! 状态码: {}", res.status()),
            Err(e) => println!("连接失败: {:?}. 请检查网络连接。", e),
        }
    }
    #[tokio::test]
    async fn test_proxy() -> std::io::Result<()> {
        // 在外面创建全局唯一的 Client，内部已配置好纯 Rustls
        let client = Client::builder()
            .use_rustls_tls() // 强制使用 rustls
            .build()
            .expect("Failed to create client");

        let client_data = web::Data::new(client);

        HttpServer::new(move || {
            App::new()
                .app_data(client_data.clone())
                .service(proxy_handler)
        })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
    }
}
