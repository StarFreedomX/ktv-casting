// 测试rupnp库API
use rupnp::ssdp::{SearchTarget, URN};
use std::time::Duration;
use futures::stream::StreamExt;

const AV_TRANSPORT: URN = URN::service("schemas-upnp-org", "AVTransport", 1);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("测试rupnp API...");
    
    // 使用正确的SearchTarget构造方法
    let search_target = SearchTarget::URN(AV_TRANSPORT);
    println!("SearchTarget::URN() 成功");
    
    // 测试discover函数
    println!("开始搜索设备...");
    
    let devices_stream = rupnp::discover(
        &search_target,
        Duration::from_secs(3),
        None,
    ).await?;
    
    let devices: Vec<_> = devices_stream.collect().await;
    println!("发现 {} 个设备", devices.len());
    
    for (i, device_result) in devices.iter().enumerate() {
        match device_result {
            Ok(device) => {
                println!("设备 {}: {} ({})", i + 1, device.friendly_name(), device.device_type());
                println!("  URL: {}", device.url());
                println!("  服务数量: {}", device.services().len());
                
                for service in device.services() {
                    println!("    服务: {}", service.service_type());
                }
            }
            Err(e) => {
                println!("设备 {} 错误: {}", i + 1, e);
            }
        }
    }
    
    Ok(())
}