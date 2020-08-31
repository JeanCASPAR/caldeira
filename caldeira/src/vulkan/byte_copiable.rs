pub unsafe trait ByteCopiable {}

unsafe impl ByteCopiable for bool {}

unsafe impl ByteCopiable for char {}
unsafe impl ByteCopiable for str {}

unsafe impl ByteCopiable for u8 {}
unsafe impl ByteCopiable for u16 {}
unsafe impl ByteCopiable for u32 {}
unsafe impl ByteCopiable for u64 {}
unsafe impl ByteCopiable for u128 {}
unsafe impl ByteCopiable for usize {}

unsafe impl ByteCopiable for i8 {}
unsafe impl ByteCopiable for i16 {}
unsafe impl ByteCopiable for i32 {}
unsafe impl ByteCopiable for i64 {}
unsafe impl ByteCopiable for i128 {}
unsafe impl ByteCopiable for isize {}

unsafe impl ByteCopiable for f32 {}
unsafe impl ByteCopiable for f64 {}

unsafe impl<T: ByteCopiable> ByteCopiable for [T] {}

unsafe impl<T: ByteCopiable> ByteCopiable for [T; 0] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 1] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 2] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 3] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 4] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 5] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 6] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 7] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 8] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 9] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 10] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 11] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 12] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 13] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 14] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 15] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 16] {}

unsafe impl<T: ByteCopiable> ByteCopiable for [T; 32] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 64] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 128] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 256] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 512] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 1024] {}
unsafe impl<T: ByteCopiable> ByteCopiable for [T; 2048] {}

unsafe impl<A: ByteCopiable, B: ByteCopiable> ByteCopiable for (A, B) {}
