//! 通用工具函数

/// 从B站URL中提取BV号与参数
pub fn extract_bv_id(url: &str) -> String {
    if let Some(start) = url.find("bilibili://video/") {
        let after_prefix = &url[start + "bilibili://video/".len()..];
        after_prefix.to_string().replace("?", "-").replace("=", "")
    } else {
        url.to_string().replace("?", "-").replace("=", "")
    }
}

/// 从错误消息中提取3位数字错误码
///
/// 例如："HTTP 404" -> Some(404)
pub fn extract_error_code(error_msg: &str) -> Option<u32> {
    error_msg
        .split(|c: char| !c.is_numeric())
        .find(|s| s.len() == 3)
        .and_then(|s| s.parse().ok())
}

/// 判断错误码是否为2xx（成功类）
///
/// 2xx状态码在UPnP/DLNA中通常表示操作成功
pub fn is_success_code(code: u32) -> bool {
    code / 100 == 2
}

/// 通用重试逻辑：执行异步操作，直到成功或达到最大重试次数
///
/// # 参数
/// - `operation_name`: 操作名称（用于日志）
/// - `max_retries`: 最大重试次数（0表示无限重试）
/// - `delay_ms`: 重试延迟（毫秒）
/// - `f`: 要执行的操作闭包
///
/// # 返回
/// - `Ok(T)`: 操作成功返回的结果
/// - `Err(String)`: 操作失败的错误信息
pub async fn retry_async<T, F, Fut>(
    operation_name: &str,
    max_retries: usize,
    delay_ms: u64,
    mut f: F,
) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(result) => {
                if retries > 0 {
                    log::info!("{}操作成功（重试{}次后）", operation_name, retries);
                }
                return Ok(result);
            }
            Err(e) => {
                let error_msg = e.to_string();
                
                retries += 1;
                if max_retries > 0 && retries > max_retries {
                    log::error!("{}失败，已达最大重试次数: {}", operation_name, error_msg);
                    return Err(format!("{}失败: {}", operation_name, error_msg));
                }

                log::warn!("{}失败: {}，{}ms后重试（第{}次）", operation_name, error_msg, delay_ms, retries);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

/// 重试直到成功的无限重试版本
pub async fn retry_until_success<T, F, Fut>(
    operation_name: &str,
    delay_ms: u64,
    f: F,
) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    retry_async(operation_name, 0, delay_ms, f).await
}

/// 检查错误是否应该被视为成功（2xx错误码）
///
/// 在UPnP/DLNA协议中，某些设备可能返回2xx错误码但实际上是成功的
pub fn should_treat_as_upnp_success(error_msg: &str) -> bool {
    if let Some(code) = extract_error_code(error_msg) {
        return is_success_code(code);
    }
    false
}
