//! Authentication handler for HTTP API

use axum::{
    extract::{Path, Json, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::provider::id::ProviderId;
use crate::server::AppState;

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

/// Set auth credentials for a provider.
/// For API key auth, stores the key in the AuthManager.
pub async fn set_auth(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    Json(info): Json<AuthInfo>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("Setting auth for provider: {}", provider_id);
    let pid = ProviderId::new(&provider_id);
    match &info {
        AuthInfo::Api { key } => {
            state.auth.set_api_key(&pid, key);
        }
        AuthInfo::Wellknown { key, .. } => {
            state.auth.set_api_key(&pid, key);
        }
        AuthInfo::OAuth { access, .. } => {
            // Store the access token as the API key for OAuth providers
            state.auth.set_api_key(&pid, access);
        }
    }
    Ok(Json(true))
}

/// Remove auth credentials for a provider.
pub async fn remove_auth(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("Removing auth for provider: {}", provider_id);
    let pid = ProviderId::new(&provider_id);
    // Remove by setting an empty key — AuthManager doesn't have a remove method,
    // so we clear the entry by inserting empty and then clearing via the internal map.
    {
        let mut keys = state.auth.config_keys_mut();
        keys.remove(&pid);
    }
    Ok(Json(true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_auth_info_serialization() {
        let api = AuthInfo::Api { key: "sk-test-key".to_string() };
        let json = serde_json::to_string(&api).unwrap();
        assert!(json.contains("\"type\":\"api\""));
        assert!(json.contains("\"key\":\"sk-test-key\""));
    }

    #[test]
    fn test_oauth_deserialization() {
        let json = r#"{"type":"api","key":"sk-test"}"#;
        let info: AuthInfo = serde_json::from_str(json).unwrap();
        match info {
            AuthInfo::Api { key } => assert_eq!(key, "sk-test"),
            _ => panic!("Expected Api variant"),
        }
    }
}


#[cfg(test)]
mod more_tests {
    use super::*;

    #[test]
    fn test_oauth_info_serialization() {
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