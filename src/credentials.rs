use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Credentials {
    Basic(String, String),
    Bearer(String),
}

impl Credentials {
    pub fn as_header(&self) -> String {
        match self {
            Credentials::Basic(username, password) => {
                format!(
                    "Basic {}",
                    base64::encode(format!("{}:{}", username, password)) // username, password
                )
            }
            Credentials::Bearer(token) => format!("Bearer {}", token),
        }
    }
}
