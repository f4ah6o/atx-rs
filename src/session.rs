use crate::error::{AtxError, Result};
use std::env;
use std::fmt;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

/// セッションID: `<unix_ns>-<pid>`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(String);

impl SessionId {
    /// 新しいセッションIDを生成: `<unix_ns>-<pid>`
    pub fn generate() -> Self {
        let unix_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let pid = process::id();
        SessionId(format!("{}-{}", unix_ns, pid))
    }

    /// 環境変数 ATX_SESSION からパース
    pub fn from_env() -> Result<Self> {
        let val = env::var("ATX_SESSION")
            .map_err(|_| AtxError::SessionNotSet)?;
        // 形式バリデーション: <数字>-<数字>
        if !val.contains('-') || val.split('-').any(|s| s.is_empty()) {
            return Err(AtxError::InvalidSessionId(val));
        }
        Ok(SessionId(val))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// セッションIDからPIDを抽出
    /// フォーマット: `<unix_ns>-<pid>`
    pub fn extract_pid(&self) -> Option<u32> {
        self.0
            .rsplit('-')
            .next()
            .and_then(|pid_str| pid_str.parse::<u32>().ok())
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug)]
pub struct Session {
    pub id: SessionId,
    pub pid: u32,
}

impl Session {
    pub fn new() -> Self {
        let id = SessionId::generate();
        let pid = process::id();
        Session { id, pid }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_format() {
        let id = SessionId::generate();
        let s = id.as_str();
        // 形式: <数字>-<数字>
        assert!(s.contains('-'));
        let parts: Vec<&str> = s.split('-').collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].chars().all(|c| c.is_ascii_digit()));
        assert!(parts[1].chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_session_from_env_missing() {
        // ATX_SESSIONが未設定の場合
        unsafe { env::remove_var("ATX_SESSION") };
        assert!(matches!(SessionId::from_env(), Err(AtxError::SessionNotSet)));
    }

    #[test]
    fn test_extract_pid() {
        let id = SessionId::generate();
        let extracted = id.extract_pid();
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap(), process::id());
    }

    #[test]
    fn test_extract_pid_from_known_format() {
        let id = SessionId("1234567890-42".to_string());
        assert_eq!(id.extract_pid(), Some(42));
    }
}
