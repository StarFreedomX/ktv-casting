use std::net::IpAddr;
use std::time::Duration;
use tokio;

mod dlna_controller;
use dlna_controller::{DlnaController, generate_didl_metadata, get_local_ip};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DLNA控制器测试 ===");
    
    // 测试1: 获取本地IP
    println!("\n1. 测试获取本地IP地址...");
    match get_local_ip().await {
        Ok(ip) => println!("   本地IP地址: {}", ip),
        Err(e) => println!("   获取IP失败: {}", e),
    }
    
    // 测试2: 生成DIDL元数据
    println!("\n2. 测试生成DIDL元数据...");
    let metadata = generate_didl_metadata("测试视频", "video/mp4", Some("0:01:30"));
    println!("   生成的元数据:");
    println!("   {}", metadata);
    
    // 测试3: 创建控制器
    println!("\n3. 创建DLNA控制器...");
    let controller = DlnaController::new();
    
    // 测试4: 发现设备
    println!("\n4. 测试发现DLNA设备...");
    match controller.discover_devices().await {
        Ok(devices) => {
            if devices.is_empty() {
                println!("   未找到DLNA设备");
            } else {
                println!("   找到 {} 个DLNA设备:", devices.len());
                for (i, device) in devices.iter().enumerate() {
                    println!("   [{}] {} - {}", i + 1, device.friendly_name, device.location);
                    println!("       支持的服务: {:?}", device.services);
                }
                
                // 如果有设备，测试基本功能
                if let Some(device) = devices.first() {
                    println!("\n5. 测试设备信息获取...");
                    
                    // 测试获取传输信息
                    match controller.get_transport_info(device).await {
                        Ok(_) => println!("   获取传输信息成功"),
                        Err(e) => println!("   获取传输信息失败: {}", e),
                    }
                    
                    // 测试获取位置信息
                    match controller.get_position_info(device).await {
                        Ok(_) => println!("   获取位置信息成功"),
                        Err(e) => println!("   获取位置信息失败: {}", e),
                    }
                }
            }
        }
        Err(e) => println!("   设备发现失败: {}", e),
    }
    
    println!("\n✅ 测试完成！");
    Ok(())
}