//! Authentication handler for HTTP API
//!
//! Implements auth credential management endpoints:
//! - PUT /auth/:providerID - Set auth credentials
//! - DELETE /auth/:providerID - Remove auth credentials

use axum::{
    extract::{Path, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

/// OAuth authentication info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthInfo {
    #[serde(rename = "type")]
    pub auth_type: String, // "oauth"
    pub refresh: String,
    pub access: String,
    pub expires: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enterprise_url: Option<String>,
}

/// API key authentication info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiAuthInfo {
    #[serde(rename = "type")]
    pub auth_type: String, // "api"
    pub key: String,
}

/// Well-known authentication info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WellKnownAuthInfo {
    #[serde(rename = "type")]
    pub auth_type: String, // "wellknown"
    pub key: String,
    pub token: String,
}

/// Auth info union type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthInfo {
    OAuth {
        refresh: String,
        access: String,
        expires: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        account_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        enterprise_url: Option<String>,
    },
    Api {
        key: String,
    },
    Wellknown {
        key: String,
        token: String,
    },
}

/// Set auth credentials for a provider
pub async fn set_auth(
    Path(provider_id): Path<String>,
    Json(_info): Json<AuthInfo>,
) -> Result<Json<bool>, StatusCode> {
    // TODO: Implement actual auth storage
    // For now, just acknowledge the request
    tracing::info!("Setting auth for provider: {}", provider_id);
    Ok(Json(true))
}

/// Remove auth credentials for a provider
pub async fn remove_auth(
    Path(provider_id): Path<String>,
) -> Result<Json<bool>, StatusCode> {
    // TODO: Implement actual auth removal
    // For now, just acknowledge the request
    tracing::info!("Removing auth for provider: {}", provider_id);
    Ok(Json(true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_info_serialization() {
        let oauth = AuthInfo::OAuth {
            refresh: "refresh_token".to_string(),
            access: "access_token".to_string(),
            expires: 3600,
            account_id: Some("account123".to_string()),
            enterprise_url: None,
        };

        let json = serde_json::to_string(&oauth).unwrap();
        assert!(json.contains("\"type\":\"oauth\""));
        assert!(json.contains("\"refresh\":\"refresh_token\""));
    }

    #[test]
    fn test_api_auth_info_serialization() {
        let api = AuthInfo::Api {
            key: "sk-test-key".to_string(),
        };

        let json = serde_json::to_string(&api).unwrap();
        assert!(json.contains("\"type\":\"api\""));
        assert!(json.contains("\"key\":\"sk-test-key\""));
    }

    #[test]
    fn test_wellknown_auth_info_serialization() {
        let wellknown = AuthInfo::Wellknown {
            key: "my-key".to_string(),
            token: "my-token".to_string(),
        };

        let json = serde_json::to_string(&wellknown).unwrap();
        assert!(json.contains("\"type\":\"wellknown\""));
    }

    #[test]
    fn test_auth_info_deserialization() {
        let json = r#"{"type":"api","key":"sk-test"}"#;
        let info: AuthInfo = serde_json::from_str(json).unwrap();
        
        match info {
            AuthInfo::Api { key } => assert_eq!(key, "sk-test"),
            _ => panic!("Expected Api variant"),
        }
    }

    #[test]
    fn test_oauth_deserialization_with_optional_fields() {
        let json = r#"{
            "type": "oauth",
            "refresh": "refresh",
            "access": "access",
            "expires": 7200,
            "account_id": "acc123"
        }"#;
        
        let info: AuthInfo = serde_json::from_str(json).unwrap();
        
        match info {
            AuthInfo::OAuth { 
                refresh, 
                access, 
                expires, 
                account_id, 
                enterprise_url 
            } => {
                assert_eq!(refresh, "refresh");
                assert_eq!(access, "access");
                assert_eq!(expires, 7200);
                assert_eq!(account_id, Some("acc123".to_string()));
                assert!(enterprise_url.is_none());
            }
            _ => panic!("Expected OAuth variant"),
        }
    }
}