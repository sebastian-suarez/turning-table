//! Byte-wise AVR C ABI primitives. The generic compiler runtime aligns and
//! copies whole machine words, which costs hundreds of bytes on this 8-bit MCU.
//! Volatile accesses keep LLVM from rewriting these loops into calls to themselves.

#[no_mangle]
unsafe extern "C" fn memcpy(
    destination: *mut core::ffi::c_void,
    source: *const core::ffi::c_void,
    count: usize,
) -> *mut core::ffi::c_void {
    let mut target = destination.cast::<u8>();
    let mut input = source.cast::<u8>();
    let mut remaining = count;
    while remaining != 0 {
        // SAFETY: the ABI caller supplies count nonoverlapping valid bytes.
        unsafe {
            target.write_volatile(input.read_volatile());
            target = target.add(1);
            input = input.add(1);
        }
        remaining -= 1;
    }
    destination
}

#[no_mangle]
unsafe extern "C" fn memset(
    destination: *mut core::ffi::c_void,
    value: core::ffi::c_int,
    count: usize,
) -> *mut core::ffi::c_void {
    for i in 0..count {
        // SAFETY: the C ABI caller guarantees count writable bytes. Only the
        // low byte of value is part of memset's specified operation.
        unsafe { destination.cast::<u8>().add(i).write_volatile(value as u8) };
    }
    destination
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_primitives_preserve_guards_and_return_the_destination() {
        let source: [u8; 6] = [0, 1, 127, 128, 254, 255];
        let mut destination = [0x5a_u8; 10];
        let pointer = unsafe { destination.as_mut_ptr().add(2).cast() };
        unsafe {
            assert_eq!(
                memcpy(pointer, source.as_ptr().cast(), source.len()),
                pointer
            );
        }
        assert_eq!(&destination[2..8], &source);
        assert_eq!(destination[0..2], [0x5a; 2]);
        assert_eq!(destination[8..], [0x5a; 2]);
        unsafe {
            assert_eq!(memset(pointer, -1, 6), pointer);
            memcpy(pointer, source.as_ptr().cast(), 0);
            memset(pointer, 0, 0);
        }
        assert_eq!(&destination[2..8], &[255; 6]);
        assert_eq!(destination[0..2], [0x5a; 2]);
        assert_eq!(destination[8..], [0x5a; 2]);
    }
}
