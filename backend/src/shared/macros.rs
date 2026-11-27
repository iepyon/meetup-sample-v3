//! 文字列で表現されるドメイン列挙型を定義するマクロ（`as_str` / `parse` / `ALL`）。

#[macro_export]
macro_rules! string_enum {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: &'static [&'static str] = &[$($text),+];

            pub fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }

            pub fn parse(value: &str) -> Option<Self> {
                match value { $($text => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}
