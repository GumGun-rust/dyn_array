use thiserror::Error;


#[derive(Error, Debug)]
pub enum Error{
    #[error("data type is 0 bytes long")]
    EmptyDataType,
    #[error("call to mmap failed")]
    MmapFail,
    #[error("call to mremap failed")]
    MremapFail,
}


