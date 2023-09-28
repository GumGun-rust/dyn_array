#[cfg(test)]
mod tests;

mod errors;

use std::{
    slice,
    fmt::{
        self,
        Debug,
        Formatter,
    },
    ffi::c_void,
    marker::PhantomData,
    ptr::NonNull,
    mem::size_of,
    num::NonZeroUsize,
    os::fd::BorrowedFd,
    ops::{
        Index,
        IndexMut,
    },
};

use nix::{
    sys::mman::{
        mmap,
        mremap,
    },
    /*
    unistd::{
        sysconf,
        SysconfVar,
    },
    */
};

pub struct ArrayConstructor{}

/*
impl ArrayConstructor {
    pub fn construct<T>() -> Result<Array<T>, errors::Error> {
        let page_size = sysconf(SysconfVar::PAGE_SIZE).unwrap().unwrap();
        Array::<T, {page_size}>::new();
        todo!();
    }
}
*/

pub struct Array<T, const N: usize = 4096>{
    byte_len: usize,
    elem_len: usize,
    byte_capacity: usize,
    start: NonNull<T>,
    phantom: PhantomData<T>,
}

impl<T> Array<T> {
    
    pub fn new() -> Result<Self, errors::Error> {
        Self::new_unchecked(64)
    }
    
    pub fn with_capacity(size:usize) -> Result<Self, errors::Error> { 
        let size = size - 1;
        let size = size as f32;
        let exponent = size.log2()as u32 +1;
        let base: i32 = 2;
        let size = usize::try_from(base.pow(exponent)).unwrap();
        Self::new_unchecked(size)
    }
    
    fn new_unchecked(size:usize) -> Result<Self, errors::Error> {
        use nix::sys::mman::ProtFlags;
        use nix::sys::mman::MapFlags;
        
        let data_size = size_of::<T>();
        
        let byte_cap = NonZeroUsize::new(usize::from((((size*data_size-1) >> 12)+1)<<12)).ok_or(errors::Error::EmptyDataType)?;
        
        let memory_holder = unsafe{mmap(
            None,
            byte_cap.into(),
            ProtFlags::PROT_READ | ProtFlags::PROT_WRITE,
            MapFlags::MAP_PRIVATE | MapFlags::MAP_ANONYMOUS,
            None::<BorrowedFd>,
            0
        )}.or(Err(errors::Error::MmapFail))?;
        
        let memory_holder_nn = unsafe{NonNull::new_unchecked(memory_holder as *mut T)};
        
        Ok(Self{
            byte_len:0,
            elem_len:0,
            byte_capacity:byte_cap.into(),
            start: memory_holder_nn,
            phantom: PhantomData,
        })
        
    }


    pub fn push(&mut self, data:T) -> Result<Option<*mut u8>, errors::Error> {
        let ret_holder:Option<*mut u8> = if self.byte_len+size_of::<T>() > self.byte_capacity {
            use nix::sys::mman::MRemapFlags;
            let old_byte_capacity = self.byte_capacity;
            self.byte_capacity <<= 1;
            
            self.start = unsafe{NonNull::new_unchecked(mremap(
                self.start.as_ptr() as *mut c_void,
                old_byte_capacity,
                self.byte_capacity,
                MRemapFlags::MREMAP_MAYMOVE,
                None
            ).or(Err(errors::Error::MremapFail))? as *mut T)};
            
            Some(self.start.as_ptr() as *mut u8)
        } else {
            None
        };
        
        let holder = unsafe{self.start.as_ptr().add(self.elem_len)};
        let pivot_mut = unsafe{NonNull::new_unchecked(holder).as_mut()};
        *pivot_mut = data;
        self.elem_len += 1;
        self.byte_len += size_of::<T>();
        Ok(ret_holder)
    }
    
    pub fn len(&self) -> usize {
        self.elem_len
    }
    
    pub fn pop(&mut self) -> Result<(), errors::Error> {
        if self.byte_len-size_of::<T>() < self.byte_capacity>>2 {
            if self.byte_capacity > 1<<12 {
                println!("shrink");
                use nix::sys::mman::MRemapFlags;
                let old_byte_capacity = self.byte_capacity;
                self.byte_capacity >>= 1;
                
                self.start = unsafe{NonNull::new_unchecked(mremap(
                    self.start.as_ptr() as *mut c_void,
                    old_byte_capacity,
                    self.byte_capacity,
                    MRemapFlags::MREMAP_MAYMOVE,
                    None
                ).or(Err(errors::Error::MremapFail))? as *mut T)};
            }
        }
        self.elem_len -= 1;
        self.byte_len -= size_of::<T>();
        Ok(())
    }
    
    pub fn base(&self) -> NonNull<T> {
        self.start
    }
    
    pub fn allocate(&mut self) -> Result<(Option<*mut u8>, NonNull<T>), errors::Error> {
        let ret_holder:Option<*mut u8> = if self.byte_len+size_of::<T>() > self.byte_capacity {
            use nix::sys::mman::MRemapFlags;
            let old_byte_capacity = self.byte_capacity;
            self.byte_capacity <<= 1;
            
            self.start = unsafe{NonNull::new_unchecked(mremap(
                self.start.as_ptr() as *mut c_void,
                old_byte_capacity,
                self.byte_capacity,
                MRemapFlags::MREMAP_MAYMOVE,
                None
            ).or(Err(errors::Error::MremapFail))? as *mut T)};
            
            Some(self.start.as_ptr() as *mut u8)
        } else {
            None
        };
        
        let holder = unsafe{self.start.as_ptr().add(self.elem_len)};
        let pivot = unsafe{NonNull::new_unchecked(holder)};
        self.elem_len += 1;
        self.byte_len += size_of::<T>();
        Ok((ret_holder, pivot))
    }
    
    pub fn get_ptr_mut(&mut self, index:usize) -> *mut T {
        unsafe{self.start.as_ptr().add(index)}
        
    }
    
    pub fn try_get(&self, index:usize) -> Result<&T, errors::Error> {
        if index>self.elem_len {
            return Err(errors::Error::OutOfRangeIndex);
        }
        Ok(&self[index])
    }

    pub fn try_get_mut(&mut self, index:usize) -> Result<&mut T, errors::Error> {
        if index>self.elem_len {
            return Err(errors::Error::OutOfRangeIndex);
        }
        Ok(&mut self[index])
    }
}

impl<T> Index<usize> for Array<T> {
    type Output = T;
    
    fn index(&self, index:usize) -> &Self::Output {
        let ptr = unsafe{self.start.as_ptr().add(index)};
        unsafe{ptr.as_ref().unwrap()}
    }
}

impl<T> IndexMut<usize> for Array<T> {
    
    fn index_mut(&mut self, index:usize) -> &mut Self::Output {
        let ptr = unsafe{self.start.as_ptr().add(index)};
        unsafe{ptr.as_mut().unwrap()}
    }
}

impl<T:Debug> Debug for Array<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let data_holder = unsafe{slice::from_raw_parts(self.start.as_ptr(), self.elem_len)};
        f.debug_struct("Array")
           .field("elem_len", &self.elem_len)
           .field("byte_len", &self.byte_len)
           .field("byte_capacity", &format!("0x{:X}", self.byte_capacity))
           .field("content", &self.start)
           .field("content", &&data_holder[..])
           .finish()
    }
}

