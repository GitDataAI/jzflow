pub const KILOBYTE: i64 = 1 << 10;
pub const MEGABYTE: i64 = 1 << 20;
pub const GIGABYTE: i64 = 1 << 30;

pub trait SmartSize {
    fn to_smart_string(self) -> String;
}

impl SmartSize for i64 {
    fn to_smart_string(self) -> String {
        let divisor = match self {
            size if size < MEGABYTE => KILOBYTE,
            size if size < GIGABYTE => MEGABYTE,
            _ => GIGABYTE,
        };
        let name = match self {
            size if size < MEGABYTE => "KB",
            size if size < GIGABYTE => "MB",
            _ => "GB",
        };

        let size = (self as f32) / (divisor as f32);
        format!("{:.2} {}", size, name)
    }
}

impl SmartSize for u64 {
    fn to_smart_string(self) -> String {
        (self as i64).to_smart_string()
    }
}

impl SmartSize for i32 {
    fn to_smart_string(self) -> String {
        (self as i64).to_smart_string()
    }
}

impl SmartSize for u32 {
    fn to_smart_string(self) -> String {
        (self as i64).to_smart_string()
    }
}
