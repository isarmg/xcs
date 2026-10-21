#[cfg(test)]
mod test {
    #[test]
    fn test() {
        span_range!(1);
        span_range!((1, 2));
    }
}
