use std::path::Path;

pub struct MemoryMappedSlice<T> {
    mmap: memmap::Mmap,
    _marker: core::marker::PhantomData<T>,
}

impl<T> MemoryMappedSlice<T> {
    pub fn from_file<P>(bin_path: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = std::fs::File::open(bin_path)?;

        let index = Self {
            mmap: unsafe { memmap::Mmap::map(&file)? },
            _marker: core::marker::PhantomData,
        };

        Ok(index)
    }

    pub fn as_slice(&self) -> &[T] {
        let slice: &[u8] = &self.mmap;
        let element_width = slice.len() / std::mem::size_of::<T>();
        let ptr = slice.as_ptr();

        unsafe { std::slice::from_raw_parts(ptr as *const _, element_width) }
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }
}
