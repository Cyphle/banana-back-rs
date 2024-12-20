use crate::security::oidc::OidcConfig;

pub fn get_oidc_config() -> OidcConfig {
    return OidcConfig::new(
        "http://localhost:8181/realms/Banana".to_string(),
        "banana".to_string(),
        "banana-secret".to_string(),
        "http://localhost:9000/login".to_string()
    )
}