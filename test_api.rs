// 简单测试rupnp API
use rupnp::ssdp::{SearchTarget, URN};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 测试不同的SearchTarget构造方法
    println!("测试SearchTarget API...");
    
    // 尝试不同的SearchTarget构造方法
    let _st1 = SearchTarget::DeviceType("test".to_string());
    let _st2 = SearchTarget::ServiceType("test".to_string());
    
    println!("SearchTarget构造成功");
    
    Ok(())
}