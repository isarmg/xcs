use crate::algorithm::Printer;
use rust_syntax::Lifetime;

impl Printer {
    pub fn lifetime(&mut self, lifetime: &Lifetime) {
        self.word("'");
        self.ident(&lifetime.ident);
    }
}
