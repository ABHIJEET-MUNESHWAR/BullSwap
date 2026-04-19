use actix_web::dev::ServiceRequest;
use actix_web::Error;

/// Extract or generate a request ID for tracing.
pub fn extract_request_id(req: &ServiceRequest) -> String {
    req.headers()
        .get("X-Request-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

/// Middleware-style function to validate an API key if configured.
///
/// If `expected_key` is None, all requests are allowed.
/// If set, requests must include `Authorization: Bearer <key>`.
pub fn validate_api_key(req: &ServiceRequest, expected_key: Option<&str>) -> Result<(), Error> {
    let expected = match expected_key {
        Some(k) => k,
        None => return Ok(()), // No auth required
    };

    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            if token == expected {
                Ok(())
            } else {
                Err(actix_web::error::ErrorUnauthorized("Invalid API key"))
            }
        }
        _ => Err(actix_web::error::ErrorUnauthorized(
            "Missing or invalid Authorization header",
        )),
    }
}
