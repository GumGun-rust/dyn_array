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
    start: *mut T,
    phantom: PhantomData<T>,
}

impl<T:Debug> Array<T> {
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
        
        Ok(Self{
            byte_len:0,
            elem_len:0,
            byte_capacity:byte_cap.into(),
            start: memory_holder as *mut T,
            phantom: PhantomData,
        })
        
    }


    pub fn push(&mut self, data:T) -> Result<(), errors::Error> {
        if self.byte_len+size_of::<T>() > self.byte_capacity {
            use nix::sys::mman::MRemapFlags;
            let old_byte_capacity = self.byte_capacity;
            self.byte_capacity <<= 1;
            
            self.start = unsafe{mremap(
                self.start as *mut c_void,
                old_byte_capacity,
                self.byte_capacity,
                MRemapFlags::MREMAP_MAYMOVE,
                None
            )}.or(Err(errors::Error::MremapFail))? as *mut T;
        }
        
        let holder = unsafe{self.start.add(self.elem_len)};
        let pivot_mut = unsafe{NonNull::new_unchecked(holder).as_mut()};
        *pivot_mut = data;
        self.elem_len += 1;
        self.byte_len += size_of::<T>();
        Ok(())
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
                
                self.start = unsafe{mremap(
                    self.start as *mut c_void,
                    old_byte_capacity,
                    self.byte_capacity,
                    MRemapFlags::MREMAP_MAYMOVE,
                    None
                )}.or(Err(errors::Error::MremapFail))? as *mut T;
            }
        }
        self.elem_len -= 1;
        self.byte_len -= size_of::<T>();
        Ok(())
    }
    
    
    fn sleep() {
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf);
    }
    
}

impl<T> Debug for Array<T> {
    
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        //todo!();
        //let data_holder = unsafe{slice::from_raw_parts(self.start, self.elem_len)};
        f.debug_struct("Array")
           .field("elem_len", &self.elem_len)
           .field("byte_len", &self.byte_len)
           .field("byte_capacity", &format!("0x{:X}", self.byte_capacity))
           .field("content", &self.start)
           //.field("content", &&data_holder[..])
           .finish()
    }
    
}
