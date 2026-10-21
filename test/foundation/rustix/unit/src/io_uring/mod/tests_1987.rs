#[cfg(test)]
mod tests {
    use super::*;
    use crate::fd::AsRawFd as _;

    /// Check that our custom structs and unions have the same layout as the
    /// kernel's versions.
    #[test]
    fn io_uring_layouts() {
        use sys as c;

        // `io_uring_ptr` is a replacement for `u64`.
        static_assertions::assert_eq_size!(io_uring_ptr, u64);
        static_assertions::assert_eq_align!(io_uring_ptr, u64);

        // Test that pointers are stored in `io_uring_ptr` in the way that
        // io_uring stores them in a `u64`.
        unsafe {
            const MAGIC: u64 = !0x0123_4567_89ab_cdef;
            let ptr = io_uring_ptr::new(MAGIC as usize as *mut c_void);
            assert_eq!(ptr.ptr, MAGIC as usize as *mut c_void);
            #[cfg(target_pointer_width = "16")]
            assert_eq!(ptr.__pad16, 0);
            #[cfg(any(target_pointer_width = "16", target_pointer_width = "32"))]
            assert_eq!(ptr.__pad32, 0);
            let int = core::mem::transmute::<io_uring_ptr, u64>(ptr);
            assert_eq!(int, MAGIC as usize as u64);
        }

        // `io_uring_user_data` is a replacement for `u64`.
        static_assertions::assert_eq_size!(io_uring_user_data, u64);
        static_assertions::assert_eq_align!(io_uring_user_data, u64);

        // Test that `u64`s and pointers are properly stored in
        // `io_uring_user_data`.
        unsafe {
            const MAGIC: u64 = !0x0123_4567_89ab_cdef;
            let user_data = io_uring_user_data::from_u64(MAGIC);
            assert_eq!(user_data.u64_(), MAGIC);
            assert_eq!(
                core::mem::transmute::<io_uring_user_data, u64>(user_data),
                MAGIC
            );
            let user_data = io_uring_user_data::from_ptr(MAGIC as usize as *mut c_void);
            assert_eq!(user_data.ptr(), MAGIC as usize as *mut c_void);
            assert_eq!(
                core::mem::transmute::<io_uring_user_data, u64>(user_data),
                MAGIC as usize as u64
            );
        }

        check_renamed_type!(off_or_addr2_union, io_uring_sqe__bindgen_ty_1);
        check_renamed_type!(addr_or_splice_off_in_union, io_uring_sqe__bindgen_ty_2);
        check_renamed_type!(addr3_or_cmd_union, io_uring_sqe__bindgen_ty_6);
        check_renamed_type!(op_flags_union, io_uring_sqe__bindgen_ty_3);
        check_renamed_type!(buf_union, io_uring_sqe__bindgen_ty_4);
        check_renamed_type!(
            splice_fd_in_or_file_index_or_addr_len_union,
            io_uring_sqe__bindgen_ty_5
        );
        check_renamed_type!(addr_len_struct, io_uring_sqe__bindgen_ty_5__bindgen_ty_1);
        check_renamed_type!(
            register_or_sqe_op_or_sqe_flags_union,
            io_uring_restriction__bindgen_ty_1
        );

        check_renamed_type!(addr3_struct, io_uring_sqe__bindgen_ty_6__bindgen_ty_1);
        check_renamed_type!(cmd_op_struct, io_uring_sqe__bindgen_ty_1__bindgen_ty_1);

        check_type!(io_uring_sqe);
        check_struct_field!(io_uring_sqe, opcode);
        check_struct_field!(io_uring_sqe, flags);
        check_struct_field!(io_uring_sqe, ioprio);
        check_struct_field!(io_uring_sqe, fd);
        check_struct_renamed_field!(io_uring_sqe, off_or_addr2, __bindgen_anon_1);
        check_struct_renamed_field!(io_uring_sqe, addr_or_splice_off_in, __bindgen_anon_2);
        check_struct_field!(io_uring_sqe, len);
        check_struct_renamed_field!(io_uring_sqe, op_flags, __bindgen_anon_3);
        check_struct_field!(io_uring_sqe, user_data);
        check_struct_renamed_field!(io_uring_sqe, buf, __bindgen_anon_4);
        check_struct_field!(io_uring_sqe, personality);
        check_struct_renamed_field!(
            io_uring_sqe,
            splice_fd_in_or_file_index_or_addr_len,
            __bindgen_anon_5
        );
        check_struct_renamed_field!(io_uring_sqe, addr3_or_cmd, __bindgen_anon_6);

        check_type!(io_uring_restriction);
        check_struct_field!(io_uring_restriction, opcode);
        check_struct_renamed_field!(
            io_uring_restriction,
            register_or_sqe_op_or_sqe_flags,
            __bindgen_anon_1
        );
        check_struct_field!(io_uring_restriction, resv);
        check_struct_field!(io_uring_restriction, resv2);

        check_struct!(io_uring_cqe, user_data, res, flags, big_cqe);
        check_struct!(
            io_uring_params,
            sq_entries,
            cq_entries,
            flags,
            sq_thread_cpu,
            sq_thread_idle,
            features,
            wq_fd,
            resv,
            sq_off,
            cq_off
        );
        check_struct!(
            io_sqring_offsets,
            head,
            tail,
            ring_mask,
            ring_entries,
            flags,
            dropped,
            array,
            resv1,
            user_addr
        );
        check_struct!(
            io_cqring_offsets,
            head,
            tail,
            ring_mask,
            ring_entries,
            overflow,
            cqes,
            flags,
            resv1,
            user_addr
        );
        check_struct!(io_uring_recvmsg_out, namelen, controllen, payloadlen, flags);
        check_struct!(io_uring_probe, last_op, ops_len, resv, resv2, ops);
        check_struct!(io_uring_probe_op, op, resv, flags, resv2);
        check_struct!(io_uring_files_update, offset, resv, fds);
        check_struct!(io_uring_rsrc_register, nr, flags, resv2, data, tags);
        check_struct!(io_uring_rsrc_update, offset, resv, data);
        check_struct!(io_uring_rsrc_update2, offset, resv, data, tags, nr, resv2);
        check_struct!(
            io_uring_getevents_arg,
            sigmask,
            sigmask_sz,
            min_wait_usec,
            ts
        );
        check_struct!(iovec, iov_base, iov_len);
        check_struct!(open_how, flags, mode, resolve);
        check_struct!(io_uring_buf_reg, ring_addr, ring_entries, bgid, flags, resv);
        check_struct!(io_uring_buf, addr, len, bid, resv);
        check_struct!(
            io_uring_sync_cancel_reg,
            addr,
            fd,
            flags,
            timeout,
            opcode,
            pad,
            pad2
        );

        check_renamed_type!(tail_or_bufs_struct, io_uring_buf_ring__bindgen_ty_1);
        check_renamed_type!(
            buf_ring_tail_struct,
            io_uring_buf_ring__bindgen_ty_1__bindgen_ty_1
        );
        check_renamed_type!(
            buf_ring_bufs_struct,
            io_uring_buf_ring__bindgen_ty_1__bindgen_ty_2
        );
        check_struct_renamed_field!(io_uring_buf_ring, tail_or_bufs, __bindgen_anon_1);

        check_struct!(
            io_uring_napi,
            busy_poll_to,
            prefer_busy_poll,
            opcode,
            pad,
            op_param,
            resv
        );
        check_struct!(
            io_uring_clone_buffers,
            src_fd,
            flags,
            src_off,
            dst_off,
            nr,
            pad
        );
        check_struct!(
            io_uring_reg_wait,
            ts,
            min_wait_usec,
            flags,
            sigmask,
            sigmask_sz,
            pad,
            pad2
        );

        check_renamed_struct!(
            MsgHdr,
            msghdr,
            msg_name,
            msg_namelen,
            msg_iov,
            msg_iovlen,
            msg_control,
            msg_controllen,
            msg_flags
        );
    }

    #[test]
    fn test_io_uring_register_files_skip() {
        use crate::backend::c;
        assert!(IORING_REGISTER_FILES_SKIP.as_raw_fd() != -1);
        assert!(IORING_REGISTER_FILES_SKIP.as_raw_fd() != c::STDIN_FILENO);
        assert!(IORING_REGISTER_FILES_SKIP.as_raw_fd() != c::STDOUT_FILENO);
        assert!(IORING_REGISTER_FILES_SKIP.as_raw_fd() != c::STDERR_FILENO);
    }
}
