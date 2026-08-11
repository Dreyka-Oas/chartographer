use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("configuration manquante ou invalide : {0}")]
    Config(String),
    #[error("{provider} a refusé l'authentification : {detail}")]
    Auth { provider: String, detail: String },
    #[error("{provider} indisponible : {detail}")]
    Remote { provider: String, detail: String },
    #[error("données incohérentes : {0}")]
    Data(String),
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Config(_) => "config",
            AppError::Auth { .. } => "auth",
            AppError::Remote { .. } => "remote",
            AppError::Data(_) => "data",
        }
    }

    pub fn remote(provider: &str, detail: impl Into<String>) -> Self {
        AppError::Remote {
            provider: provider.into(),
            detail: detail.into(),
        }
    }

    pub fn auth(provider: &str, detail: impl Into<String>) -> Self {
        AppError::Auth {
            provider: provider.into(),
            detail: detail.into(),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Data(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Data(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
