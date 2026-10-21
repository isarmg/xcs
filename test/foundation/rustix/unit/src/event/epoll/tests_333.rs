#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::c;

    #[test]
    fn test_epoll_layouts() {
        check_renamed_type!(Event, epoll_event);
        check_renamed_struct_renamed_field!(Event, epoll_event, flags, events);
        #[cfg(libc)]
        check_renamed_struct_renamed_field!(Event, epoll_event, data, u64);
        #[cfg(not(libc))]
        check_renamed_struct_renamed_field!(Event, epoll_event, data, data);
    }
}
