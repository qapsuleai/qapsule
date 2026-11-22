//! Unit tests of Marla's words and counts (moved out of `marla.rs`).

use super::*;

#[test]
fn number_words() {
    assert_eq!(words_tr(5), "beş");
    assert_eq!(words_tr(20), "yirmi");
    assert_eq!(words_tr(120), "yüz yirmi");
    assert_eq!(words_tr(15), "on beş");
    assert_eq!(words_en(5), "five");
    assert_eq!(words_en(120), "one hundred twenty");
    assert_eq!(words_en(25), "twenty-five");
    assert_eq!(mult(2.5, Lang::Tr, true), "iki buçuk");
    assert_eq!(mult(2.0, Lang::En, true), "two");
    assert_eq!(mult(2.5, Lang::Tr, false), "2,5");
}

#[test]
fn sentences_are_counted_like_the_reader() {
    assert_eq!(sentence_count("A 1,5. B; C"), 3);
    assert_eq!(sentence_count("Fiyat 148,90."), 1);
    assert_eq!(sentence_count(""), 0);
}

#[test]
fn percentages() {
    assert_eq!(pct(150.2, 148.9, Lang::Tr), "%0,9");
    assert_eq!(pct(150.2, 148.9, Lang::En), "0.9%");
    assert_eq!(pct(148.91, 148.9, Lang::Tr), "%0,1'den az");
}
