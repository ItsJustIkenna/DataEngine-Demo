use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    error: Vec<String>,
    result: TokenResultData,
}

impl TokenResponse {
    pub fn from_json(json: &serde_json::Value) -> Result<TokenResponse> {
        let token_response: TokenResponse = serde_json::from_value(json.clone())?;
        Ok(token_response)
    }

    pub fn get_token(&self) -> String {
        self.result.token.clone()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenResultData {
    token: String,
    expires: u64,
}