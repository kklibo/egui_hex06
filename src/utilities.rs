use egui::Color32;
use std::ops::Deref;

pub fn byte_color(byte: u8) -> Color32 {
    let r = byte & 0b11000000;
    let g = (byte & 0b00111000) << 2;
    let b = (byte & 0b00000111) << 5;

    Color32::from_rgb(r, g, b)
}

pub fn contrast(color: Color32) -> Color32 {
    Color32::from_rgb(
        u8::wrapping_add(color.r(), 128),
        u8::wrapping_add(color.g(), 128),
        u8::wrapping_add(color.b(), 128),
    )
}

pub fn diff_color(diff_bytes: Option<usize>, count: u64) -> Color32 {
    if let Some(diff_bytes) = diff_bytes {
        if diff_bytes == 0 {
            Color32::from_rgb(127, 127, 127)
        } else {
            let diff = 255.0 * (1.0 - (diff_bytes as f32 / count as f32));
            Color32::from_rgb(255, diff as u8, diff as u8)
        }
    } else {
        Color32::BLACK
    }
}

pub fn semantic_color(value: u8) -> Color32 {
    Color32::from_rgb(value, value, value)
}

pub fn diff_at_index(
    data0: &Option<impl Deref<Target = [u8]>>,
    data1: &Option<impl Deref<Target = [u8]>>,
    index: usize,
) -> Option<usize> {
    if let (Some(data0), Some(data1)) = (data0, data1) {
        if let (Some(d0), Some(d1)) = (data0.get(index), data1.get(index)) {
            return Some(if d0 == d1 { 0 } else { 1 });
        }
    }

    None
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub enum ByteString {
    Exact { value: u64, label: String },
    Approximate { value: f64, label: String },
}

#[allow(dead_code)]
pub fn byte_string_binary(value: u64, verbose: bool) -> ByteString {
    enum Unit {
        TB,
        GB,
        MB,
        KB,
        B,
    }
    impl Unit {
        fn bytes(&self) -> u64 {
            match self {
                Self::TB => 1024 * 1024 * 1024 * 1024,
                Self::GB => 1024 * 1024 * 1024,
                Self::MB => 1024 * 1024,
                Self::KB => 1024,
                Self::B => 1,
            }
        }
        fn label(&self) -> &'static str {
            match self {
                Self::TB => "TB",
                Self::GB => "GB",
                Self::MB => "MB",
                Self::KB => "KB",
                Self::B => "B",
            }
        }
        fn label_verbose(&self) -> &'static str {
            match self {
                Self::TB => "Terabytes",
                Self::GB => "Gigabytes",
                Self::MB => "Megabytes",
                Self::KB => "Kilobytes",
                Self::B => "Bytes",
            }
        }
        fn label_verbose_singular(&self) -> &'static str {
            match self {
                Self::TB => "Terabyte",
                Self::GB => "Gigabyte",
                Self::MB => "Megabyte",
                Self::KB => "Kilobyte",
                Self::B => "Byte",
            }
        }
    }

    let unit = if value >= Unit::TB.bytes() {
        Unit::TB
    } else if value >= Unit::GB.bytes() {
        Unit::GB
    } else if value >= Unit::MB.bytes() {
        Unit::MB
    } else if value >= Unit::KB.bytes() {
        Unit::KB
    } else {
        Unit::B
    };

    let label = if verbose {
        if value == unit.bytes() {
            unit.label_verbose_singular()
        } else {
            unit.label_verbose()
        }
    } else {
        unit.label()
    };

    if value % unit.bytes() == 0 {
        ByteString::Exact {
            value: value / unit.bytes(),
            label: label.to_string(),
        }
    } else {
        ByteString::Approximate {
            value: value as f64 / unit.bytes() as f64,
            label: label.to_string(),
        }
    }
}

#[allow(dead_code)]
pub fn byte_string_decimal(value: u64) -> ByteString {
    if value < 10 {
        return ByteString::Exact {
            value,
            label: "B".to_string(),
        };
    }

    let e_value = value.ilog10();
    let factor = 10u64.pow(e_value);
    if factor % value == 0 {
        ByteString::Exact {
            value: value / factor,
            label: format!("e{e_value}B"),
        }
    } else {
        ByteString::Approximate {
            value: value as f64 / factor as f64,
            label: format!("e{e_value}B"),
        }
    }
}

#[allow(dead_code)]
pub fn byte_string_decimal_verbose(value: u64) -> ByteString {
    enum Unit {
        E12,
        E9,
        E6,
        E3,
        E0,
    }

    impl Unit {
        fn bytes(&self) -> u64 {
            match self {
                Self::E12 => 10u64.pow(12),
                Self::E9 => 10u64.pow(9),
                Self::E6 => 10u64.pow(6),
                Self::E3 => 10u64.pow(3),
                Self::E0 => 1,
            }
        }
        fn label(&self) -> &'static str {
            match self {
                Self::E12 => "Trillion Bytes",
                Self::E9 => "Billion Bytes",
                Self::E6 => "Million Bytes",
                Self::E3 => "Thousand Bytes",
                Self::E0 => "Bytes",
            }
        }
    }

    let unit = if value >= Unit::E12.bytes() {
        Unit::E12
    } else if value >= Unit::E9.bytes() {
        Unit::E9
    } else if value >= Unit::E6.bytes() {
        Unit::E6
    } else if value >= Unit::E3.bytes() {
        Unit::E3
    } else {
        Unit::E0
    };

    let label = if value == 1 { "Byte" } else { unit.label() };

    if value % unit.bytes() == 0 {
        ByteString::Exact {
            value: value / unit.bytes(),
            label: label.to_string(),
        }
    } else {
        ByteString::Approximate {
            value: value as f64 / unit.bytes() as f64,
            label: label.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_string_binary() {
        fn print_byte_string(a: ByteString) {
            match a {
                ByteString::Exact { value, label } => println!("{} {}", value, label),
                ByteString::Approximate { value, label } => println!("~{:0.2} {}", value, label),
            }
        }

        print_byte_string(byte_string_binary(0, false));
        print_byte_string(byte_string_binary(0, true));
        print_byte_string(byte_string_binary(1, false));
        print_byte_string(byte_string_binary(1, true));
        print_byte_string(byte_string_binary(1024, false));
        print_byte_string(byte_string_binary(1024, true));
        print_byte_string(byte_string_binary(1025, false));
        print_byte_string(byte_string_binary(1025, true));
        print_byte_string(byte_string_binary(1024 * 1024, false));
        print_byte_string(byte_string_binary(1024 * 1024, true));
        print_byte_string(byte_string_binary(1024 * 1024 + 1, false));
        print_byte_string(byte_string_binary(1024 * 1024 + 1, true));
        print_byte_string(byte_string_binary(1024 * 1024 * 1024, false));
        print_byte_string(byte_string_binary(1024 * 1024 * 1024, true));
        print_byte_string(byte_string_binary(1024 * 1024 * 1024 + 1, false));
        print_byte_string(byte_string_binary(1024 * 1024 * 1024 + 1, true));
        print_byte_string(byte_string_binary(1024 * 1024 * 1024 * 1024, false));
        print_byte_string(byte_string_binary(1024 * 1024 * 1024 * 1024, true));
        print_byte_string(byte_string_binary(1024 * 1024 * 1024 * 1024 + 1, false));
        print_byte_string(byte_string_binary(1024 * 1024 * 1024 * 1024 + 1, true));
        print_byte_string(byte_string_binary(1024 * 1024 * 1024 * 1024 * 1024, false));
        print_byte_string(byte_string_binary(1024 * 1024 * 1024 * 1024 * 1024, true));
        print_byte_string(byte_string_binary(
            1024 * 1024 * 1024 * 1024 * 1024 + 1,
            false,
        ));
        print_byte_string(byte_string_binary(
            1024 * 1024 * 1024 * 1024 * 1024 + 1,
            true,
        ));

        print_byte_string(byte_string_binary(123456789, false));
        print_byte_string(byte_string_binary(123456789, true));
    }

    #[test]
    fn test_byte_string_decimal() {
        fn print_byte_string(a: ByteString) {
            match a {
                ByteString::Exact { value, label } => println!("{}{}", value, label),
                ByteString::Approximate { value, label } => println!("~{:0.2}{}", value, label),
            }
        }

        print_byte_string(byte_string_decimal(0));
        print_byte_string(byte_string_decimal(1));
        print_byte_string(byte_string_decimal(10));
        print_byte_string(byte_string_decimal(10 + 1));
        print_byte_string(byte_string_decimal(100));
        print_byte_string(byte_string_decimal(100 + 1));
        print_byte_string(byte_string_decimal(1000));
        print_byte_string(byte_string_decimal(1000 + 1));
        print_byte_string(byte_string_decimal(10000000));
        print_byte_string(byte_string_decimal(10000000 + 1));
        print_byte_string(byte_string_decimal(123456789));
    }

    #[test]
    fn test_byte_string_decimal_verbose() {
        fn print_byte_string(a: ByteString) {
            match a {
                ByteString::Exact { value, label } => println!("{} {}", value, label),
                ByteString::Approximate { value, label } => println!("~{:0.2} {}", value, label),
            }
        }

        print_byte_string(byte_string_decimal_verbose(0));
        print_byte_string(byte_string_decimal_verbose(1));
        print_byte_string(byte_string_decimal_verbose(1000));
        print_byte_string(byte_string_decimal_verbose(1000 + 1));
        print_byte_string(byte_string_decimal_verbose(1000 * 1000));
        print_byte_string(byte_string_decimal_verbose(1000 * 1000 + 1));
        print_byte_string(byte_string_decimal_verbose(1000 * 1000 * 1000));
        print_byte_string(byte_string_decimal_verbose(1000 * 1000 * 1000 + 1));
        print_byte_string(byte_string_decimal_verbose(1000 * 1000 * 1000 * 1000));
        print_byte_string(byte_string_decimal_verbose(1000 * 1000 * 1000 * 1000 + 1));
        print_byte_string(byte_string_decimal_verbose(
            1000 * 1000 * 1000 * 1000 * 1000,
        ));
        print_byte_string(byte_string_decimal_verbose(
            1000 * 1000 * 1000 * 1000 * 1000 + 1,
        ));

        print_byte_string(byte_string_decimal_verbose(123456789));
    }
}
