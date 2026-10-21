#[cfg(test)]
mod tests {
    use super::*;

    use core::{mem, ptr};

    unsafe fn raw_fill_copied<T: Copy>(dst: *mut T, len: usize, val: T) {
        if mem::size_of::<T>() == 0 {
            return;
        }

        if len == 0 {
            return;
        }

        if mem::size_of::<T>() == 1 {
            let val: u8 = mem::transmute_copy(&val);
            dst.write_bytes(val, len);
        } else {
            dst.write(val);

            let mut n = 1;
            while n <= len / 2 {
                ptr::copy_nonoverlapping(dst, dst.add(n), n);
                n *= 2;
            }

            let count = len - n;
            if count > 0 {
                ptr::copy_nonoverlapping(dst, dst.add(n), count);
            }
        }
    }

    fn fill<T: Copy>(mut buf: Out<'_, [T]>, val: T) -> &'_ mut [T] {
        unsafe {
            let len = buf.len();
            let dst = buf.as_mut_ptr();
            raw_fill_copied(dst, len, val);
            buf.assume_init()
        }
    }

    #[test]
    fn fill_vec() {
        for n in 0..128 {
            let mut v: Vec<u32> = Vec::with_capacity(n);
            fill(v.spare_capacity_mut().as_out(), 0x12345678);
            unsafe { v.set_len(n) };
            for &x in &v {
                assert_eq!(x, 0x12345678);
            }
            drop(v);
        }
    }
}
