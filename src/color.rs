/// 背景色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Reset,
    Hex(&'static str),
}

impl Color {
    /// OSC 11シーケンスに変換
    pub fn to_osc_set_sequence(&self) -> String {
        match self {
            Color::Reset => String::new(), // ResetはOSC 111を使用
            Color::Hex(hex) => format!("\x1b]11;{}\x07", hex),
        }
    }

    /// OSC 111（リセット）シーケンス
    pub fn to_osc_reset_sequence() -> &'static str {
        "\x1b]111\x07"
    }

    /// イベント用の定義済み色
    pub const PERMISSION_RED: Color = Color::Hex("#4a0000");
    pub const IDLE_YELLOW: Color = Color::Hex("#3b2a00");
    pub const STOP_BLUE: Color = Color::Hex("#002b36");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorAction {
    Set(Color),
    Reset,
    NoOp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_set_sequence() {
        assert_eq!(
            Color::PERMISSION_RED.to_osc_set_sequence(),
            "\x1b]11;#4a0000\x07"
        );
        assert_eq!(
            Color::IDLE_YELLOW.to_osc_set_sequence(),
            "\x1b]11;#3b2a00\x07"
        );
        assert_eq!(
            Color::STOP_BLUE.to_osc_set_sequence(),
            "\x1b]11;#002b36\x07"
        );
    }

    #[test]
    fn test_osc_reset_sequence() {
        assert_eq!(Color::to_osc_reset_sequence(), "\x1b]111\x07");
    }
}
