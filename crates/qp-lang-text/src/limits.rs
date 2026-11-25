//! Size limits of the text surface. They guard the hand-written recursive parser, the
//! lowering and the printer against hostile input (deep nesting, huge files, huge
//! tokens). The defaults are the language's structural caps; the registry task supplies
//! the values from parameter keys, passed in through [`lower_with`](crate::lower_with) /
//! [`print_with`](crate::print_with). The printer uses the same `max_depth` as the
//! parser, so what it prints always lowers again.

/// Size limits of one text file and of one printed program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextLimits {
    /// Characters of the source at most (`E_TOO_LARGE`).
    pub max_source_chars: usize,
    /// Characters of one token (name, number, string) at most (`E_TOO_LARGE`).
    pub max_token_chars: usize,
    /// Expression nesting at most, parentheses included (`E_TOO_LARGE`); the printer
    /// stops inlining past it (the node gets its own line).
    pub max_depth: usize,
}

/// Default source size: 64 Ki characters, eight times the default canonical IR size
/// (`program.max_ir_bytes` = 8192) for comments and spelling.
pub const DEFAULT_MAX_SOURCE_CHARS: usize = 65_536;
/// Default token size (names are at most 64 characters; strings are short texts).
pub const DEFAULT_MAX_TOKEN_CHARS: usize = 1024;
/// Default nesting depth; far above any real formula, far below a stack overflow.
pub const DEFAULT_MAX_DEPTH: usize = 64;

impl Default for TextLimits {
    fn default() -> TextLimits {
        TextLimits {
            max_source_chars: DEFAULT_MAX_SOURCE_CHARS,
            max_token_chars: DEFAULT_MAX_TOKEN_CHARS,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}
