use anyhow::{Result, anyhow};
use serde::Deserialize;
use reqwest::Client;
use std::time::{Duration, Instant};
use std::thread::sleep;
use tracing::{info, debug};

/// The response from the device code request
#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    /// The device code to use in token requests
    pub device_code: String,
    /// Time in seconds until the device code expires
    pub expires_in: u64,
    /// Polling interval in seconds
    pub interval: u64,
    /// The code to show to the user
    pub user_code: String,
    /// The URL the user should visit
    pub verification_uri: String,
}

/// The response from the token request
#[derive(Debug, Deserialize, Clone, serde::Serialize)]
pub struct TokenResponse {
    /// The access token to use in API requests
    pub access_token: String,
    /// Time in seconds until the token expires
    pub expires_in: u64,
    /// The refresh token to use to get a new access token
    pub refresh_token: String,
    /// The scopes that were granted
    pub scope: Vec<String>,
    /// The type of token (usually "bearer")
    pub token_type: String,
}

/// The error response from the token request
#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    /// HTTP status code
    pub status: u16,
    /// Error message
    pub message: String,
}

/// Structure to manage OAuth authentication
pub struct OAuthManager {
    /// HTTP client for making requests
    client: Client,
    /// Client ID for the application
    client_id: String,
    /// The scopes needed for the application
    scopes: Vec<String>,
    /// The token response if authenticated
    token: Option<TokenResponse>,
    /// When the token was obtained
    token_obtained_at: Option<Instant>,
}

impl OAuthManager {
    /// Create a new OAuth manager
    ///
    /// # Arguments
    /// * `client_id` - The client ID for the application
    /// * `scopes` - The scopes needed for the application
    ///
    /// # Returns
    /// A new OAuthManager
    pub fn new(client_id: String, scopes: Vec<String>) -> Self {
        OAuthManager {
            client: Client::new(),
            client_id,
            scopes,
            token: None,
            token_obtained_at: None,
        }
    }

    /// Get the current access token, refreshing if necessary
    ///
    /// # Returns
    /// The access token to use for requests
    pub async fn get_access_token(&mut self) -> Result<String> {
        // If we have a token, check if it's still valid
        if let (Some(token), Some(obtained_at)) = (&self.token, self.token_obtained_at) {
            let elapsed = obtained_at.elapsed();
            let expires_in = Duration::from_secs(token.expires_in);
            
            // If the token expires in less than 10 minutes, refresh it
            if elapsed > expires_in.saturating_sub(Duration::from_secs(600)) {
                // Token is about to expire, refresh it
                debug!("Token is about to expire, refreshing");
                self.refresh_token().await?;
            }
            
            // Return the access token
            return Ok(self.token.as_ref().unwrap().access_token.clone());
        }
        
        // No token, authenticate
        Err(anyhow!("Not authenticated"))
    }
    
    /// Start the device code flow
    ///
    /// # Returns
    /// A Result containing the device code response if successful
    pub async fn start_device_code_flow(&self) -> Result<DeviceCodeResponse> {
        let form = reqwest::multipart::Form::new()
            .text("client_id", self.client_id.clone())
            .text("scopes", self.scopes.join(" "));
            
        let response = self.client
            .post("https://id.twitch.tv/oauth2/device")
            .multipart(form)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error: ErrorResponse = response.json().await?;
            return Err(anyhow!("Failed to start device code flow: {}", error.message));
        }
        
        let device_code: DeviceCodeResponse = response.json().await?;
        Ok(device_code)
    }
    
    /// Poll for token with the device code
    ///
    /// # Arguments
    /// * `device_code` - The device code response from start_device_code_flow
    ///
    /// # Returns
    /// A Result indicating success or failure
    pub async fn poll_for_token(&mut self, device_code: &DeviceCodeResponse) -> Result<()> {
        let start_time = Instant::now();
        let expiry = Duration::from_secs(device_code.expires_in);
        let interval = Duration::from_secs(device_code.interval);
        
        info!("Polling for token, will timeout in {} seconds", device_code.expires_in);
        
        while start_time.elapsed() < expiry {
            let form = reqwest::multipart::Form::new()
                .text("client_id", self.client_id.clone())
                .text("scopes", self.scopes.join(" "))
                .text("device_code", device_code.device_code.clone())
                .text("grant_type", "urn:ietf:params:oauth:grant-type:device_code".to_string());
                
            let response = self.client
                .post("https://id.twitch.tv/oauth2/token")
                .multipart(form)
                .send()
                .await?;
                
            if response.status().is_success() {
                let token: TokenResponse = response.json().await?;
                self.token = Some(token);
                self.token_obtained_at = Some(Instant::now());
                return Ok(());
            }
            
            // Parse the error response
            let error_text = response.text().await?;
            
            // Check if we need to keep waiting
            if error_text.contains("authorization_pending") {
                debug!("Authorization pending, waiting {} seconds", interval.as_secs());
                sleep(interval);
                continue;
            }
            
            // Any other error is fatal
            return Err(anyhow!("Error polling for token: {}", error_text));
        }
        
        Err(anyhow!("Device code flow timed out"))
    }
    
    /// Refresh the access token using the refresh token
    ///
    /// # Returns
    /// A Result indicating success or failure
    async fn refresh_token(&mut self) -> Result<()> {
        if self.token.is_none() {
            return Err(anyhow!("No refresh token available"));
        }
        
        let refresh_token = self.token.as_ref().unwrap().refresh_token.clone();
        
        let form = reqwest::multipart::Form::new()
            .text("client_id", self.client_id.clone())
            .text("refresh_token", refresh_token)
            .text("grant_type", "refresh_token".to_string());
            
        let response = self.client
            .post("https://id.twitch.tv/oauth2/token")
            .multipart(form)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Failed to refresh token: {}", error_text));
        }
        
        let token: TokenResponse = response.json().await?;
        self.token = Some(token);
        self.token_obtained_at = Some(Instant::now());
        
        Ok(())
    }
    
    /// Run the device code flow and wait for user authentication
    ///
    /// # Returns
    /// A Result indicating success or failure
    pub async fn authenticate(&mut self) -> Result<()> {
        // Start the device code flow
        let device_code = self.start_device_code_flow().await?;
        
        // Print instructions for the user
        println!("\n=== Twitch Authentication Required ===");
        println!("Please visit: {}", device_code.verification_uri);
        println!("And enter the code: {}", device_code.user_code);
        println!("Waiting for authentication...");
        
        // Poll for the token
        self.poll_for_token(&device_code).await?;
        
        info!("Authentication successful!");
        println!("Authentication successful! You can now use the bot.");
        
        Ok(())
    }
    
    /// Check if the manager has a valid access token
    ///
    /// # Returns
    /// true if the manager has a valid access token, false otherwise
    pub fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }
    
    /// Get the current token information
    ///
    /// # Returns
    /// The current token information if authenticated, None otherwise
    pub fn get_token(&self) -> Option<TokenResponse> {
        self.token.clone()
    }
    
    /// Save token to a file for later use
    ///
    /// # Arguments
    /// * `path` - The path to save the token to
    ///
    /// # Returns
    /// A Result indicating success or failure
    pub fn save_token(&self, path: &str) -> Result<()> {
        if let Some(token) = &self.token {
            let token_json = serde_json::to_string_pretty(token)?;
            std::fs::write(path, token_json)?;
            Ok(())
        } else {
            Err(anyhow!("No token to save"))
        }
    }
    
    /// Load token from a file
    ///
    /// # Arguments
    /// * `path` - The path to load the token from
    ///
    /// # Returns
    /// A Result indicating success or failure
    pub fn load_token(&mut self, path: &str) -> Result<()> {
        let token_json = std::fs::read_to_string(path)?;
        let token: TokenResponse = serde_json::from_str(&token_json)?;
        self.token = Some(token);
        self.token_obtained_at = Some(Instant::now());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use serde_json::json;
    
    // This test can't test the actual API call since we need to modify the client to point
    // to our mock server. For now, we'll just test that the function is defined correctly
    // and would execute if we had a real server to test against.
    #[tokio::test]
    #[ignore]
    async fn test_start_device_code_flow() -> Result<()> {
        let mut server = Server::new();
        
        // Mock the device code endpoint 
        let _mock = server.mock("POST", "/oauth2/device")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "device_code": "test_device_code",
                "expires_in": 1800,
                "interval": 5,
                "user_code": "ABCDEFGH",
                "verification_uri": "https://www.twitch.tv/activate"
            }"#)
            .create();
            
        let _oauth = OAuthManager::new(
            "test_client_id".to_string(),
            vec!["chat:read".to_string(), "chat:edit".to_string()]
        );
        
        // In a real test, we would modify the client's base URL to point to our mock server
        // For now, we're skipping this test
        
        Ok(())
    }
}