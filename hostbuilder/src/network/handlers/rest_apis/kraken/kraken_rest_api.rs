use anyhow::{
    anyhow,
    Result
};
use base64::{
    prelude::BASE64_STANDARD,
    Engine
};
use hmac::{
    Hmac,
    Mac
};
use reqwest::{
    header::{
        HeaderMap,
        HeaderValue,
        CONTENT_TYPE
    },
    Client,
};
use sha2::{
    Digest,
    Sha256,
    Sha512
};
use std::{
    env,
    time::{
        SystemTime,
        UNIX_EPOCH,
        Instant
    }
};
use url::Url;

use crate::network::handlers::{
    models::kraken::{responses::{parse_message, Response}, token_response::TokenResponse},
    logging::{KRAKEN_REST_LOGGER, AuthEvent},
};
use crate::infrastructure::logging_facade::KRAKEN_LOGGER;
use crate::{log_info, log_error, log_warn};

pub struct KrakenRestHandler;

impl KrakenRestHandler {
    pub async fn authenticate(endpoint: &str) -> Result<TokenResponse> {
        let auth_start = Instant::now();
        
        // Use nanoseconds for higher precision to avoid nonce collisions across multiple pods
        // Kraken requires strictly increasing nonces, and microsecond precision prevents collisions
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos() as u64; // Nanosecond precision (u64 safe until year 2554)

        let url = Url::parse(endpoint).expect("Failed to parse URL");

        let api_key = env::var("KRAKEN_API_KEY").unwrap_or_default();
        let secret_key = env::var("KRAKEN_SECRET_KEY").unwrap_or_default();
        
        // Check if credentials are valid before attempting authentication
        if secret_key.is_empty() || secret_key == "placeholder_secret_key" {
            log_warn!(KRAKEN_LOGGER, "Kraken API credentials not configured (placeholder or empty). Skipping authentication.");
            return Err(anyhow::anyhow!("API credentials not configured"));
        }

        // Create JSON body with nonce (matching Python implementation)
        let body_json = format!(r#"{{"nonce":"{}"}}"#, nonce);
        
        let signature = generate_kraken_signature(url.path(), &nonce.to_string(), &body_json, &secret_key)?;
        let client = Client::new();

        let body = body_json;
        // Create the headers
        let mut headers = HeaderMap::new();
        headers.insert("API-Key", HeaderValue::from_str(&api_key)?);
        headers.insert("API-Sign", HeaderValue::from_str(&signature)?);
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        
        match client.post(url).headers(headers).body(body).send().await {
            Ok(res) => {
                let auth_latency = auth_start.elapsed().as_micros() as u64;
                let body = res.text().await.expect("Failed to parse API response");

                match parse_message(&body) {
                    Ok(Response::TokenResponse(api_response)) => {
                        log_info!(KRAKEN_LOGGER, "Successfully received TokenResponse");
                        
                        // Log successful authentication
                        let auth_event = AuthEvent {
                            endpoint: endpoint.to_string(),
                            success: true,
                            error: None,
                            latency_micros: Some(auth_latency),
                        };
                        let _ = KRAKEN_REST_LOGGER.log_auth_event(auth_event).await;
                        
                        Ok(api_response)
                    }
                    Ok(other_response) => {
                        // Return an error because we expected a TokenResponse
                        let error_msg = format!("Expected TokenResponse but got {:?}", other_response);
                        
                        // Log authentication error
                        let auth_event = AuthEvent {
                            endpoint: endpoint.to_string(),
                            success: false,
                            error: Some(error_msg.clone()),
                            latency_micros: Some(auth_latency),
                        };
                        let _ = KRAKEN_REST_LOGGER.log_auth_event(auth_event).await;
                        
                        Err(anyhow!(error_msg))
                    }
                    Err(e) => {
                        // Handle parsing error
                        log_error!(KRAKEN_LOGGER, "Failed to parse authentication response: {}", e);
                        
                        // Log parsing error
                        let auth_event = AuthEvent {
                            endpoint: endpoint.to_string(),
                            success: false,
                            error: Some(format!("Failed to parse response: {}", e)),
                            latency_micros: Some(auth_latency),
                        };
                        let _ = KRAKEN_REST_LOGGER.log_auth_event(auth_event).await;
                        
                        Err(e.into())
                    }
                }
            }
            Err(e) => {
                log_error!(KRAKEN_LOGGER, "Kraken API request error: {}", e);
                
                // Log network error
                let auth_event = AuthEvent {
                    endpoint: endpoint.to_string(),
                    success: false,
                    error: Some(format!("Network error: {}", e)),
                    latency_micros: None,
                };
                let _ = KRAKEN_REST_LOGGER.log_auth_event(auth_event).await;
                
                Err(e.into())
            }
        }
    }
}

type HmacSha512 = Hmac<Sha512>;
fn generate_kraken_signature(
    api_path: &str,
    nonce: &str,
    body_data: &str,
    secret_key: &str,
) -> Result<String> {
    // 1. Calculate SHA256 of the nonce + body data (matching Python: nonce + data)
    let mut sha256 = Sha256::new();
    sha256.update((nonce.to_string() + body_data).as_bytes());
    let sha256_hash = sha256.finalize();
    
    // 2. Decode the secret key from base64
    let decoded_key = BASE64_STANDARD
        .decode(secret_key)
        .map_err(|e| anyhow::anyhow!("Failed to decode API secret from base64: {}. Check if KRAKEN_SECRET_KEY is properly configured.", e))?;
    
    // 3. Create an HMAC-SHA512 instance with the decoded key
    let mut mac =
        HmacSha512::new_from_slice(&decoded_key).expect("HMAC can take key of any size");
    
    // 4. Update the HMAC with the API path and SHA256 hash (matching Python: path + hash)
    mac.update(api_path.as_bytes());
    mac.update(&sha256_hash);
    
    // 5. Finalize the HMAC calculation and encode the result into base64
    let hmac_result = mac.finalize();
    let hmac_bytes = hmac_result.into_bytes();
    Ok(BASE64_STANDARD.encode(hmac_bytes))
}
