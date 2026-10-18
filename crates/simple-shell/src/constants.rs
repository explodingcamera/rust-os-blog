pub const CTRL_L: u8 = 12;
pub const ENTER: u8 = 13;
pub const BACKSPACE: u8 = 127;
pub const CTRL_C: u8 = 3;
pub const ESCAPE: u8 = 27;

pub const CSI: u8 = 91;
pub const CSI_UP: u8 = 65;
pub const CSI_DOWN: u8 = 66;
pub const CSI_RIGHT: u8 = 67;
pub const CSI_LEFT: u8 = 68;

pub const ASCII_BACKSPACE: &str = "\x08";
pub const ANSI_CURSOR_RIGHT: &str = "\x1b[1C";
pub const ANSI_CURSOR_LEFT: &str = "\x1b[1D";
pub const ANSI_CLEAR_SCREEN: &str = "\x1b[2J\x1b[1;1H";
