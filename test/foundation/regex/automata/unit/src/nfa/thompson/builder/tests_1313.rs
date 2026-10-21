#[cfg(test)]
mod tests {
    use super::*;

    // This asserts that a builder state doesn't have its size changed. It is
    // *really* easy to accidentally increase the size, and thus potentially
    // dramatically increase the memory usage of NFA builder.
    //
    // This assert doesn't mean we absolutely cannot increase the size of a
    // builder state. We can. It's just here to make sure we do it knowingly
    // and intentionally.
    //
    // A builder state is unfortunately a little bigger than an NFA state,
    // since we really want to support adding things to a pre-existing state.
    // i.e., We use Vec<thing> instead of Box<[thing]>. So we end up using an
    // extra 8 bytes per state. Sad, but at least it gets freed once the NFA
    // is built.
    #[test]
    fn state_has_small_size() {
        #[cfg(target_pointer_width = "64")]
        assert_eq!(32, core::mem::size_of::<State>());
        #[cfg(target_pointer_width = "32")]
        assert_eq!(16, core::mem::size_of::<State>());
    }
}
