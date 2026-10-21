#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        let mut pool = super::Map::builder().keys(|_| "a").values(|_| "b").build();
        pool.service(&"hello");
    }
}
