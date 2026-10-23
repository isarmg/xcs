fn main() {
    let cell = once_cell::sync::OnceCell::<u32>::new();
    cell.get_or_init(|| {
        cell.get_or_init(|| 1);
        2
    });
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/once_cell/functions/examples/reentrant_init_deadlocks/dummy_test_9.rs"));
