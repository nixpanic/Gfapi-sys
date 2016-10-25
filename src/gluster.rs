use libc::{c_int, c_void, dev_t, mode_t, stat, strerror};
use glfs::*;

use std::error::Error as err;
use std::mem::zeroed;
use std::ffi::{CString, IntoStringError, NulError};
use std::io::Error;
use std::string::FromUtf8Error;

/// Custom error handling for the library
#[derive(Debug)]
pub enum GlusterError {
    FromUtf8Error(FromUtf8Error),
    NulError(NulError),
    Error(String),
    IoError(Error),
    IntoStringError(IntoStringError),
}

impl GlusterError {
    /// Create a new GlusterError with a String message
    fn new(err: String) -> GlusterError {
        GlusterError::Error(err)
    }

    /// Convert a GlusterError into a String representation.
    pub fn to_string(&self) -> String {
        match *self {
            GlusterError::FromUtf8Error(ref err) => err.utf8_error().to_string(),
            GlusterError::NulError(ref err) => err.description().to_string(),
            GlusterError::Error(ref err) => err.to_string(),
            GlusterError::IoError(ref err) => err.description().to_string(),
            GlusterError::IntoStringError(ref err) => err.description().to_string(),
        }
    }
}

impl From<NulError> for GlusterError {
    fn from(err: NulError) -> GlusterError {
        GlusterError::NulError(err)
    }
}

impl From<FromUtf8Error> for GlusterError {
    fn from(err: FromUtf8Error) -> GlusterError {
        GlusterError::FromUtf8Error(err)
    }
}
impl From<IntoStringError> for GlusterError {
    fn from(err: IntoStringError) -> GlusterError {
        GlusterError::IntoStringError(err)
    }
}
impl From<Error> for GlusterError {
    fn from(err: Error) -> GlusterError {
        GlusterError::IoError(err)
    }
}

fn get_error(n: c_int) -> Result<String, GlusterError> {
    unsafe {
        let error_cstring = CString::from_raw(strerror(n));
        let message = try!(error_cstring.into_string());
        Ok(message)
    }
}

pub struct Gluster {
    cluster_handle: *mut Struct_glfs,
}

impl Drop for Gluster {
    fn drop(&mut self) {
        if self.cluster_handle.is_null() {
            // No cleanup needed
            return;
        }
        unsafe {
            glfs_fini(self.cluster_handle);
        }
    }
}

impl Gluster {
    /// Connect to a Ceph cluster and return a connection handle glfs_t
    pub fn connect(volume_name: &str) -> Result<Gluster, GlusterError> {
        let vol_name = try!(CString::new(volume_name));
        unsafe {
            let cluster_handle = glfs_new(vol_name.as_ptr());
            let ret_code = glfs_init(cluster_handle);
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code))));
            }
            Ok(Gluster { cluster_handle: cluster_handle })
        }
    }

    /// Disconnect from a Gluster cluster and destroy the connection handle
    /// For clean up, this is only necessary after connect() has succeeded.
    /// Normally there is no need to call this function.  When Rust cleans
    /// up the Gluster struct it will automatically call disconnect
    pub fn disconnect(self) {
        if self.cluster_handle.is_null() {
            // No cleanup needed
            return;
        }
        unsafe {
            glfs_fini(self.cluster_handle);
        }
    }
    pub fn open(&self, path: &str, flags: i32) -> Result<*mut Struct_glfs_fd, GlusterError> {
        let path = try!(CString::new(path));
        unsafe {
            let file_handle = glfs_open(self.cluster_handle, path.as_ptr(), flags);
            Ok(file_handle)
        }
    }
    pub fn create(&self,
                  path: String,
                  flags: i32,
                  mode: mode_t)
                  -> Result<*mut Struct_glfs_fd, GlusterError> {
        let path = try!(CString::new(path));
        unsafe {
            let file_handle = glfs_creat(self.cluster_handle, path.as_ptr(), flags, mode);
            Ok(file_handle)
        }
    }
    pub fn close(file_handle: &mut Struct_glfs_fd) -> Result<(), GlusterError> {
        unsafe {
            let ret_code = glfs_close(file_handle);
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code))));
            }
        }
        Ok(())
    }
    pub fn read(file_handle: &mut Struct_glfs_fd,
                fill_buffer: &mut [u8],
                flags: i32)
                -> Result<isize, GlusterError> {
        unsafe {
            let read_size = glfs_read(file_handle,
                                      fill_buffer.as_mut_ptr() as *mut c_void,
                                      fill_buffer.len(),
                                      flags);
            if read_size < 0 {
                return Err(GlusterError::new(try!(get_error(read_size as i32))));
            }
            Ok(read_size)

        }

    }
    pub fn write(file_handle: &mut Struct_glfs_fd,
                 buffer: &[u8],
                 flags: i32)
                 -> Result<isize, GlusterError> {
        unsafe {
            let write_size = glfs_write(file_handle,
                                        buffer.as_ptr() as *const c_void,
                                        buffer.len(),
                                        flags);
            if write_size < 0 {
                return Err(GlusterError::new(try!(get_error(write_size as i32))));
            }
            Ok(write_size)
        }
    }
    pub fn readv(file_handle: &mut Struct_glfs_fd,
                 iov: &mut [&mut [u8]],
                 flags: i32)
                 -> Result<isize, GlusterError> {
        unsafe {
            let read_size = glfs_readv(file_handle,
                                       iov.as_ptr() as *const iovec,
                                       iov.len() as i32,
                                       flags);
            if read_size < 0 {
                return Err(GlusterError::new(try!(get_error(read_size as i32))));
            }
            Ok(read_size)

        }
    }
    pub fn writev(file_handle: &mut Struct_glfs_fd,
                  iov: &[&[u8]],
                  flags: i32)
                  -> Result<isize, GlusterError> {
        unsafe {
            let write_size = glfs_writev(file_handle,
                                         iov.as_ptr() as *const iovec,
                                         iov.len() as i32,
                                         flags);
            if write_size < 0 {
                return Err(GlusterError::new(try!(get_error(write_size as i32))));
            }
            Ok(write_size)

        }
    }

    pub fn pread(file_handle: &mut Struct_glfs_fd,
                 fill_buffer: &mut [u8],
                 count: usize,
                 offset: i64,
                 flags: i32)
                 -> Result<isize, GlusterError> {
        unsafe {
            let read_size = glfs_pread(file_handle,
                                       fill_buffer.as_mut_ptr() as *mut c_void,
                                       count,
                                       offset,
                                       flags);
            if read_size < 0 {
                return Err(GlusterError::new(try!(get_error(read_size as i32))));
            }
            Ok(read_size)
        }
    }
    pub fn pwrite(file_handle: &mut Struct_glfs_fd,
                  buffer: &[u8],
                  count: usize,
                  offset: i64,
                  flags: i32)
                  -> Result<isize, GlusterError> {
        unsafe {
            let write_size = glfs_pwrite(file_handle,
                                         buffer.as_ptr() as *mut c_void,
                                         count,
                                         offset,
                                         flags);
            if write_size < 0 {
                return Err(GlusterError::new(try!(get_error(write_size as i32))));
            }
            Ok(write_size)

        }
    }

    pub fn preadv(file_handle: &mut Struct_glfs_fd,
                  iov: &mut [&mut [u8]],
                  offset: i64,
                  flags: i32)
                  -> Result<isize, GlusterError> {
        unsafe {
            let read_size = glfs_preadv(file_handle,
                                        iov.as_ptr() as *const iovec,
                                        iov.len() as i32,
                                        offset,
                                        flags);
            if read_size < 0 {
                return Err(GlusterError::new(try!(get_error(read_size as i32))));
            }
            Ok(read_size)
        }
    }
    // TODO: Use C IoVec
    pub fn pwritev(file_handle: &mut Struct_glfs_fd,
                   iov: &[&[u8]],
                   offset: i64,
                   flags: i32)
                   -> Result<isize, GlusterError> {
        unsafe {
            let write_size = glfs_pwritev(file_handle,
                                          iov.as_ptr() as *const iovec,
                                          iov.len() as i32,
                                          offset,
                                          flags);
            if write_size < 0 {
                return Err(GlusterError::new(try!(get_error(write_size as i32))));
            }
            Ok(write_size)
        }
    }
    pub fn lseek(file_handle: &mut Struct_glfs_fd,
                 offset: i64,
                 whence: i32)
                 -> Result<i64, GlusterError> {
        unsafe {
            let file_offset = glfs_lseek(file_handle, offset, whence);
            if file_offset < 0 {
                return Err(GlusterError::new(try!(get_error(file_offset as i32))));
            }
            Ok(file_offset)

        }

    }
    pub fn truncate(&self, path: &str, length: i64) -> Result<(), GlusterError> {
        let path = try!(CString::new(path));

        unsafe {
            let ret_code = glfs_truncate(self.cluster_handle, path.as_ptr(), length);
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code as i32))));
            }
        }
        Ok(())
    }
    pub fn ftruncate(file_handle: &mut Struct_glfs_fd, length: i64) -> Result<(), GlusterError> {
        unsafe {
            let ret_code = glfs_ftruncate(file_handle, length);
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code as i32))));
            }
        }
        Ok(())
    }
    pub fn lsstat(&self, path: &str) -> Result<stat, GlusterError> {
        let path = try!(CString::new(path));
        unsafe {
            let mut stat_buf: stat = zeroed();
            let ret_code = glfs_lstat(self.cluster_handle, path.as_ptr(), &mut stat_buf);
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code as i32))));
            }
            Ok(stat_buf)
        }
    }
    pub fn stat(&self, path: &str) -> Result<stat, GlusterError> {
        let path = try!(CString::new(path));
        unsafe {
            let mut stat_buf: stat = zeroed();
            let ret_code = glfs_stat(self.cluster_handle, path.as_ptr(), &mut stat_buf);
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code as i32))));
            }
            Ok(stat_buf)
        }

    }
    pub fn fstat(file_handle: &mut Struct_glfs_fd) -> Result<stat, GlusterError> {
        unsafe {
            let mut stat_buf: stat = zeroed();
            let ret_code = glfs_fstat(file_handle, &mut stat_buf);
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code as i32))));
            }
            Ok(stat_buf)
        }
    }
    pub fn fsync(file_handle: &mut Struct_glfs_fd) -> Result<(), GlusterError> {
        unsafe {
            let ret_code = glfs_fsync(file_handle);
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code as i32))));
            }
        }
        Ok(())
    }

    pub fn fdatasync(file_handle: &mut Struct_glfs_fd) -> Result<(), GlusterError> {
        unsafe {
            let ret_code = glfs_fdatasync(file_handle);
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code as i32))));
            }

        }
        Ok(())
    }
    pub fn access(&self, path: &str, mode: i32) -> Result<(), GlusterError> {
        let path = try!(CString::new(path));
        unsafe {
            let ret_code = glfs_access(self.cluster_handle, path.as_ptr(), mode);
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code as i32))));
            }

        }
        Ok(())
    }

    pub fn symlink(&self, oldpath: &str, newpath: &str) -> Result<(), GlusterError> {
        let old_path = try!(CString::new(oldpath));
        let new_path = try!(CString::new(newpath));
        unsafe {
            let ret_code = glfs_symlink(self.cluster_handle, old_path.as_ptr(), new_path.as_ptr());
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code as i32))));
            }

        }
        Ok(())
    }

    pub fn readlink(&self, path: &str, buf: &mut [u8]) -> Result<(), GlusterError> {
        let path = try!(CString::new(path));
        unsafe {
            let ret_code = glfs_readlink(self.cluster_handle,
                                         path.as_ptr(),
                                         buf.as_mut_ptr() as *mut i8,
                                         buf.len());
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code))));
            }
        }
        Ok(())
    }

    pub fn mknod(&self, path: &str, mode: mode_t, dev: dev_t) -> Result<(), GlusterError> {
        let path = try!(CString::new(path));
        unsafe {
            let ret_code = glfs_mknod(self.cluster_handle, path.as_ptr(), mode, dev);
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code))));
            }

        }
        Ok(())
    }

    pub fn mkdir(&self, path: &str, mode: mode_t) -> Result<(), GlusterError> {
        let path = try!(CString::new(path));
        unsafe {
            let ret_code = glfs_mkdir(self.cluster_handle, path.as_ptr(), mode);
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code))));
            }

        }
        Ok(())
    }

    pub fn unlink(&self, path: &str) -> Result<(), GlusterError> {
        let path = try!(CString::new(path));
        unsafe {
            let ret_code = glfs_unlink(self.cluster_handle, path.as_ptr());
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code))));
            }

        }
        Ok(())
    }
    pub fn rmdir(&self, path: &str) -> Result<(), GlusterError> {
        let path = try!(CString::new(path));
        unsafe {
            let ret_code = glfs_rmdir(self.cluster_handle, path.as_ptr());
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code as i32))));
            }
        }
        Ok(())
    }
    pub fn rename(&self, oldpath: &str, newpath: &str) -> Result<(), GlusterError> {
        let old_path = try!(CString::new(oldpath));
        let new_path = try!(CString::new(newpath));
        unsafe {
            let ret_code = glfs_rename(self.cluster_handle, old_path.as_ptr(), new_path.as_ptr());
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code))));
            }
        }
        Ok(())
    }

    pub fn link(&self, oldpath: &str, newpath: &str) -> Result<(), GlusterError> {
        let old_path = try!(CString::new(oldpath));
        let new_path = try!(CString::new(newpath));
        unsafe {
            let ret_code = glfs_link(self.cluster_handle, old_path.as_ptr(), new_path.as_ptr());
            if ret_code < 0 {
                return Err(GlusterError::new(try!(get_error(ret_code))));
            }
        }
        Ok(())
    }

    pub fn opendir(&self, path: &str) -> Result<*mut Struct_glfs_fd, GlusterError> {
        let path = try!(CString::new(path));
        unsafe {
            let file_handle = glfs_opendir(self.cluster_handle, path.as_ptr());
            Ok(file_handle)
        }
    }
}
