//! Parser (PR §12.1, §12.4): tokens to statements. The token stream is first cut into
//! statements (one per line; a `{ ... }` block may span lines), then each statement is
//! parsed on its own, so a syntax error never stops the following lines.

mod cursor;
mod expr;
mod split;
mod stmt;

pub(crate) use cursor::{Cursor, PErr};
pub(crate) use split::{parse, Parsed};
