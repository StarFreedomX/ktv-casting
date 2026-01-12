use crate::dlna_controller::{DlnaController, generate_didl_metadata};
use crate::media_server::start_media_server;
use local_ip_address::local_ip;
use std::path::Path;
use tokio::time::{Duration, sleep};
// use warp::Filter;
use actix_web::{get, web, App, HttpServer, HttpResponse, Error};
use futures_util::StreamExt;
use reqwest::Client;

mod dlna_controller;
mod media_server;
mod bilibili_parser;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== KTV投屏DLNA应用启动 ===");

    // 启动媒体服务器 - 使用spawn_blocking避免Send trait问题
    let server_port = 8080;
    // 使用warp提供临时的HTTP服务用于调试
    
    // let route = warp::path("test_videos")
    // .and(warp::path("12.mp4"))
    // .and(warp::fs::file("test_videos/12.mp4"))
    // .boxed();


    // let server_handle = tokio::spawn(async move {
    //     // route 已经被 move 进来了
    //     warp::serve(route)
    //         .run(([0, 0, 0, 0], server_port))
    //         .await;
    // });

    let client = Client::builder() // 强制使用 rustls
        .tls_backend_rustls() // 强制使用 rustls
        .build()
        .expect("Failed to create client");

    let client_data = web::Data::new(client);

    let server_handle = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            match HttpServer::new(move || {
                App::new()
                    .app_data(client_data.clone())
                    .service(proxy_handler)
            })
            .bind(("0.0.0.0", server_port)) {
                Ok(server) => {
                    if let Err(e) = server.run().await {
                        eprintln!("服务器运行错误: {}", e);
                    }
                }
                Err(e) => eprintln!("服务器绑定失败: {}", e),
            }
        })
    });
    
    // 等待服务器启动
    sleep(Duration::from_secs(2)).await;

    // 获取本地IP地址
    let local_ip = local_ip()?;
    println!("本地IP地址: {}", local_ip);

    // 创建DLNA控制器
    let controller = DlnaController::new();

    // 发现DLNA设备
    println!("开始搜索DLNA设备...");
    let devices = controller.discover_devices().await?;

    if devices.is_empty() {
        println!("未发现任何DLNA设备，请确保设备在同一网络中并已开启DLNA功能");
        return Ok(());
    }

    println!("发现 {} 个DLNA设备", devices.len());

    // 选择第一个设备进行测试
    let device = &devices[0];
    println!("选择设备: {}", device.friendly_name);

    // 测试媒体文件
    let test_video = "test_videos/12.mp4";
    if !Path::new(test_video).exists() {
        println!("测试视频文件不存在: {}", test_video);
        return Ok(());
    }

    // 生成元数据
    // let metadata = generate_didl_metadata("测试视频", "video/mp4", Some("0:03:30"));
    let metadata = "".to_string();
    // 设置AVTransport URI
    println!("设置媒体URI...");
    if let Err(e) = controller
        .set_avtransport_uri(
            device,
            &format!("/{}", test_video),
            &metadata,
            local_ip,
            server_port,
        )
        .await
    {
        eprintln!("设置媒体URI失败: {}", e);
    }

    // 等待设备准备
    sleep(Duration::from_secs(2)).await;

    // 开始播放
    println!("开始播放...");
    if let Err(e) = controller.play(device).await {
        eprintln!("播放失败: {}", e);
    }

    // 播放10秒
    println!("播放10秒...");
    sleep(Duration::from_secs(10)).await;

    // 暂停
    println!("暂停播放...");
    if let Err(e) = controller.pause(device).await {
        eprintln!("暂停失败: {}", e);
    }

    // 等待2秒
    sleep(Duration::from_secs(2)).await;

    // 恢复播放
    println!("恢复播放...");
    if let Err(e) = controller.play(device).await {
        eprintln!("恢复播放失败: {}", e);
    }

    // 播放5秒
    println!("播放5秒...");
    sleep(Duration::from_secs(5)).await;

    // 停止
    println!("停止播放...");
    if let Err(e) = controller.stop(device).await {
        eprintln!("停止失败: {}", e);
    }

    // 获取设备状态
    println!("获取设备状态...");
    if let Err(e) = controller.get_transport_info(device).await {
        eprintln!("获取传输信息失败: {}", e);
    }

    if let Err(e) = controller.get_position_info(device).await {
        eprintln!("获取位置信息失败: {}", e);
    }

    println!("=== 测试完成 ===");
    println!("按Ctrl+C退出...");

    // 等待用户中断
    tokio::signal::ctrl_c().await?;

    // 关闭服务器
    server_handle.abort();

    println!("应用已退出");
    Ok(())
}

#[get("/{url:.*}")]
async fn proxy_handler(
    path: web::Path<(String,)>,
    client: web::Data<reqwest::Client>, // 明确指向 reqwest
) -> Result<HttpResponse, actix_web::Error> {
    let (url_path,) = path.into_inner();
    let target_url = bilibili_parser::get_bilibili_direct_link("BV1LS4MzKE8y", Some(1)).await.unwrap();

    let response = client
        .get(&target_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3")
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // 解决图 4：中转状态码，避开 http 库版本冲突
    let status_u16 = response.status().as_u16();
    let mut client_resp = HttpResponse::build(
        actix_web::http::StatusCode::from_u16(status_u16)
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
    );

    // 解决图 2：中转 Header
    for (name, value) in response.headers().iter() {
        let name_str = name.as_str();
        if name_str != "connection" && name_str != "content-encoding" && name_str != "transfer-encoding" {
            client_resp.insert_header((name_str, value.as_bytes()));
        }
    }

    // 解决图 1 & 3：流式转换
    let body_stream = response.bytes_stream().map(|item| {
        item.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    });

    Ok(client_resp.streaming(body_stream))
}
