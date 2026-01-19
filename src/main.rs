use crate::dlna_controller::DlnaController;
use crate::bilibili_parser::get_bilibili_direct_link;
use local_ip_address::local_ip;
use actix_web::{get, web, App, HttpServer, HttpResponse};
use futures_util::StreamExt;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Mutex;
use playlist_manager::PlaylistManager;
use anyhow::{Result, bail};
use std::io;

mod dlna_controller;
mod media_server;
mod bilibili_parser;
mod playlist_manager;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== KTV投屏DLNA应用启动 ===");
    println!("输入房间链接，如https://ktv.example.com/102");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("无法读取输入");
    let url_str = input.trim();
    // ② 拆分出 base URL
    // 找到「://」的位置，之后往右找第一个 '/'（路径开始）
    let scheme_end = match url_str.find("://") {
        Some(idx) => idx + 3,                // 跳过 "://"
        None => panic!("输入不是合法的 URL"),
    };

    let host_end_opt = url_str[scheme_end..].find('/');
    let host_end = match host_end_opt {
        Some(pos) => scheme_end + pos,      // 找到路径开始的位置
        None => url_str.len(),              // 没有任何路径，整个字符串就是 base URL
    };

    let base_url = &url_str[..host_end];
    let path_part = &url_str[host_end..];   // 可能以 '/' 开头

    // ③ 从路径中取最后一段（非空）作为 room_id
    let segments: Vec<&str> = path_part.split('/')
        .filter(|s| !s.is_empty())
        .collect();

    if segments.is_empty() {
        eprintln!("错误：没有找到房间号");
        bail!("No room id");
    }

    let room_str = segments.last().unwrap(); // 最后一个非空段
    let room_id: u64 = match room_str.parse::<u64>() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("错误：房间号不是合法的整数（{}）", room_str);
            bail!("Wrong room id");;
        }
    };


    let server_port = 8080;
    let playlist: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let mut playlist_manager = PlaylistManager::new(base_url, room_id, playlist.clone());
    

    // 1. 创建 Reqwest Client
    let client = Client::builder()
        .tls_backend_rustls()
        .build()
        .expect("Failed to create client");
    
    let client_data = web::Data::new(client);

    // 2. 配置 HttpServer，运行
    let server = HttpServer::new(move || {
        App::new()
            .app_data(client_data.clone())
            .service(proxy_handler)
    })
    .bind(("0.0.0.0", server_port))?
    .run();

    let local_ip = local_ip()?;
    let controller = DlnaController::new();
    let devices = controller.discover_devices().await?;
    if devices.is_empty() {
        bail!("No DLNA Devices");
    }
    for (i,device) in devices.iter().enumerate() {
        println!("{}: {} at {}",i ,device.friendly_name, device.location);
    }
    println!("输入设备编号：");
    input.clear();
    io::stdin().read_line(&mut input).expect("读取编号失败");
    let device_num: usize = input.parse()?;
    if device_num > devices.len() {bail!("编号有误");}
    let device = devices[device_num].clone(); // clone owned copy
    let device_cloned = device.clone();
    playlist_manager.start_periodic_update(move |url| {
        let controller = controller.clone();
        let device = device.clone();
        Box::pin(async move {
            controller.set_next_avtransport_uri(&device, &url, "", local_ip, server_port)
                .await.unwrap();
            controller.next(&device).await.unwrap();
        })
    });

    tokio::spawn(async move {
        let controller = DlnaController::new();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        let mut remaining_secs;
        loop {
            interval.tick().await;
            remaining_secs = controller.get_remaining_secs(&device_cloned).await.unwrap();
            if remaining_secs <= 2 {
                playlist_manager.next_song().await.unwrap();
            }
        }
    });
    server.await?;

    println!("应用已退出");
    Ok(())
}


#[get("/{url:.*}")]
async fn proxy_handler(
    path: web::Path<(String,)>,
    client: web::Data<reqwest::Client>,
) -> Result<HttpResponse, actix_web::Error> {
    let (origin_url,) = path.into_inner();
    let bv_id = origin_url[..origin_url.find('?').unwrap_or(origin_url.len())].to_string();
        let page: Option<u32> = if let Some(pos) = origin_url.find("?page=") {
            origin_url[pos + 6..].parse().ok()
        } else {
            None
        };
        let target_url = get_bilibili_direct_link(&bv_id, page).await.map_err(actix_web::error::ErrorInternalServerError)?;

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
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
    );

    for (name, value) in response.headers().iter() {
        let name_str = name.as_str();
        if name_str != "connection" && name_str != "content-encoding" && name_str != "transfer-encoding" {
            client_resp.insert_header((name_str, value.as_bytes()));
        }
    }

    let body_stream = response.bytes_stream().map(|item| {
        item.map_err(|e| std::io::Error::other(e))
    });

    Ok(client_resp.streaming(body_stream))
}
