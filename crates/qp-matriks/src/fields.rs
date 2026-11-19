//! Field-name tables generated from the proto descriptors, and [`FieldSet`]:
//! the set of fields one incoming message carried.

/// The fields of one proto message, as declared in `proto/*.proto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageFields {
    /// Full proto name, e.g. `messages.SymbolMessage`.
    pub message: &'static str,
    /// `(field number, proto field name)`, sorted by number.
    pub fields: &'static [(u32, &'static str)],
}

include!(concat!(env!("OUT_DIR"), "/fields.rs"));

/// Full proto name of `SymbolMessage`.
pub const SYMBOL_MESSAGE: &str = "messages.SymbolMessage";
/// Full proto name of `TimeMessage`.
pub const TIME_MESSAGE: &str = "messages.TimeMessage";
/// Full proto name of `MessengerMessage`.
pub const MESSENGER_MESSAGE: &str = "messages.MessengerMessage";

/// The field table of a message by full proto name.
pub fn message_fields(message: &str) -> Option<&'static MessageFields> {
    MESSAGE_FIELDS.iter().find(|m| m.message == message)
}

/// Proto field name of `number` in `message`.
pub fn field_name(message: &str, number: u32) -> Option<&'static str> {
    let m = message_fields(message)?;
    m.fields
        .binary_search_by_key(&number, |&(n, _)| n)
        .ok()
        .map(|i| m.fields[i].1)
}

/// Field number of the proto field `name` in `message`.
pub fn field_number(message: &str, name: &str) -> Option<u32> {
    message_fields(message)?
        .fields
        .iter()
        .find(|&&(_, n)| n == name)
        .map(|&(num, _)| num)
}

/// The fields one incoming message carried on the wire.
///
/// Numbers not declared in the proto (a newer Matriks schema) are kept;
/// [`FieldSet::unknown`] lists them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSet {
    message: &'static str,
    numbers: Vec<u32>,
}

impl FieldSet {
    /// A field set for `message` from sorted, de-duplicated field numbers.
    pub fn new(message: &'static str, mut numbers: Vec<u32>) -> Self {
        numbers.sort_unstable();
        numbers.dedup();
        FieldSet { message, numbers }
    }

    /// Full proto message name.
    pub fn message(&self) -> &'static str {
        self.message
    }

    /// Field numbers, sorted.
    pub fn numbers(&self) -> &[u32] {
        &self.numbers
    }

    /// Whether field `number` arrived.
    pub fn contains(&self, number: u32) -> bool {
        self.numbers.binary_search(&number).is_ok()
    }

    /// Whether the proto field `name` (e.g. `"dailyVolume"`) arrived.
    pub fn contains_name(&self, name: &str) -> bool {
        field_number(self.message, name).is_some_and(|n| self.contains(n))
    }

    /// Proto names of the declared fields that arrived, in field-number order.
    pub fn names(&self) -> Vec<&'static str> {
        self.numbers
            .iter()
            .filter_map(|&n| field_name(self.message, n))
            .collect()
    }

    /// Field numbers that arrived but are not declared in the proto.
    pub fn unknown(&self) -> Vec<u32> {
        self.numbers
            .iter()
            .copied()
            .filter(|&n| field_name(self.message, n).is_none())
            .collect()
    }

    /// Number of fields that arrived.
    pub fn len(&self) -> usize {
        self.numbers.len()
    }

    /// Whether no field arrived.
    pub fn is_empty(&self) -> bool {
        self.numbers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_table_matches_symbol_proto() {
        let m = message_fields(SYMBOL_MESSAGE).unwrap();
        assert_eq!(m.fields.len(), 85);
        assert_eq!(field_name(SYMBOL_MESSAGE, 1), Some("symbolId"));
        assert_eq!(field_name(SYMBOL_MESSAGE, 5), Some("bid"));
        assert_eq!(field_number(SYMBOL_MESSAGE, "dailyVolume"), Some(38));
        assert_eq!(field_number(SYMBOL_MESSAGE, "stockStatus"), Some(74));
        assert_eq!(field_number(SYMBOL_MESSAGE, "publishReason"), Some(76));
        assert_eq!(field_number(TIME_MESSAGE, "isBistPPOpen"), Some(4));
        assert_eq!(field_number(MESSENGER_MESSAGE, "subject"), Some(5));
        // Every proto message is in the table (22 files; some declare several).
        assert!(MESSAGE_FIELDS.len() >= 22);
    }

    #[test]
    fn field_set_queries() {
        let s = FieldSet::new(SYMBOL_MESSAGE, vec![5, 1, 5, 999]);
        assert_eq!(s.numbers(), &[1, 5, 999]);
        assert!(s.contains_name("bid"));
        assert!(!s.contains_name("last"));
        assert!(!s.contains_name("noSuchField"));
        assert_eq!(s.names(), vec!["symbolId", "bid"]);
        assert_eq!(s.unknown(), vec![999]);
        assert_eq!(s.len(), 3);
    }
}
