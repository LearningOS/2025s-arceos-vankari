#![allow(dead_code)]

use core::ffi::{c_void, c_char, c_int};
use axhal::arch::TrapFrame;
use axhal::trap::{register_trap_handler, SYSCALL};
use axerrno::LinuxError;
use axtask::current;
use axtask::TaskExtRef;
use axhal::paging::MappingFlags;
use arceos_posix_api as api;
use arceos_posix_api::imp::fd_ops::get_file_like;
use axhal::mem::VirtAddr;
const SYS_IOCTL: usize = 29;
const SYS_OPENAT: usize = 56;
const SYS_CLOSE: usize = 57;
const SYS_READ: usize = 63;
const SYS_WRITE: usize = 64;
const SYS_WRITEV: usize = 66;
const SYS_EXIT: usize = 93;
const SYS_EXIT_GROUP: usize = 94;
const SYS_SET_TID_ADDRESS: usize = 96;
const SYS_MMAP: usize = 222;

const AT_FDCWD: i32 = -100;

/// Macro to generate syscall body
///
/// It will receive a function which return Result<_, LinuxError> and convert it to
/// the type which is specified by the caller.
#[macro_export]
macro_rules! syscall_body {
    ($fn: ident, $($stmt: tt)*) => {{
        #[allow(clippy::redundant_closure_call)]
        let res = (|| -> axerrno::LinuxResult<_> { $($stmt)* })();
        match res {
            Ok(_) | Err(axerrno::LinuxError::EAGAIN) => debug!(concat!(stringify!($fn), " => {:?}"),  res),
            Err(_) => info!(concat!(stringify!($fn), " => {:?}"), res),
        }
        match res {
            Ok(v) => v as _,
            Err(e) => {
                -e.code() as _
            }
        }
    }};
}

bitflags::bitflags! {
    #[derive(Debug)]
    /// permissions for sys_mmap
    ///
    /// See <https://github.com/bminor/glibc/blob/master/bits/mman.h>
    struct MmapProt: i32 {
        /// Page can be read.
        const PROT_READ = 1 << 0;
        /// Page can be written.
        const PROT_WRITE = 1 << 1;
        /// Page can be executed.
        const PROT_EXEC = 1 << 2;
    }
}

impl From<MmapProt> for MappingFlags {
    fn from(value: MmapProt) -> Self {
        let mut flags = MappingFlags::USER;
        if value.contains(MmapProt::PROT_READ) {
            flags |= MappingFlags::READ;
        }
        if value.contains(MmapProt::PROT_WRITE) {
            flags |= MappingFlags::WRITE;
        }
        if value.contains(MmapProt::PROT_EXEC) {
            flags |= MappingFlags::EXECUTE;
        }
        flags
    }
}

bitflags::bitflags! {
    #[derive(Debug)]
    /// flags for sys_mmap
    ///
    /// See <https://github.com/bminor/glibc/blob/master/bits/mman.h>
    struct MmapFlags: i32 {
        /// Share changes
        const MAP_SHARED = 1 << 0;
        /// Changes private; copy pages on write.
        const MAP_PRIVATE = 1 << 1;
        /// Map address must be exactly as requested, no matter whether it is available.
        const MAP_FIXED = 1 << 4;
        /// Don't use a file.
        const MAP_ANONYMOUS = 1 << 5;
        /// Don't check for reservations.
        const MAP_NORESERVE = 1 << 14;
        /// Allocation is for a stack.
        const MAP_STACK = 0x20000;
    }
}

#[register_trap_handler(SYSCALL)]
fn handle_syscall(tf: &TrapFrame, syscall_num: usize) -> isize {
    ax_println!("handle_syscall [{}] ...", syscall_num);
    let ret = match syscall_num {
         SYS_IOCTL => sys_ioctl(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _) as _,
        SYS_SET_TID_ADDRESS => sys_set_tid_address(tf.arg0() as _),
        SYS_OPENAT => sys_openat(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _, tf.arg3() as _),
        SYS_CLOSE => sys_close(tf.arg0() as _),
        SYS_READ => sys_read(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _),
        SYS_WRITE => sys_write(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _),
        SYS_WRITEV => sys_writev(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _),
        SYS_EXIT_GROUP => {
            ax_println!("[SYS_EXIT_GROUP]: system is exiting ..");
            axtask::exit(tf.arg0() as _)
        },
        SYS_EXIT => {
            ax_println!("[SYS_EXIT]: system is exiting ..");
            axtask::exit(tf.arg0() as _)
        },
        SYS_MMAP => sys_mmap(
            tf.arg0() as _,
            tf.arg1() as _,
            tf.arg2() as _,
            tf.arg3() as _,
            tf.arg4() as _,
            tf.arg5() as _,
        ),
        _ => {
            ax_println!("Unimplemented syscall: {}", syscall_num);
            -LinuxError::ENOSYS.code() as _
        }
    };
    ret
}

#[allow(unused_variables)]
fn sys_mmap(
    addr: *mut usize,
    length: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    _offset: isize,
) -> isize  {  
      
    // 解析标志位  
    let mmap_flags = MmapFlags::from_bits_truncate(flags);  
    let prot_flags = MmapProt::from_bits_truncate(prot);  
    let mapping_flags = MappingFlags::from(prot_flags);  
      
    // 页对齐长度  
    let aligned_length = (length + 0xfff) & !0xfff;  
      
    // 获取当前任务的地址空间  
    let current_task = axtask::current();  
    let mut aspace = current_task.task_ext().aspace.lock();  
      
    if mmap_flags.contains(MmapFlags::MAP_ANONYMOUS) {  
        // 匿名映射实现  
        let start_addr = VirtAddr::from(0x10000000);  
        match aspace.map_alloc(start_addr, aligned_length, mapping_flags, true) {  
            Ok(_) => return start_addr.as_usize() as isize,  
            Err(_) => return -LinuxError::ENOMEM.code() as isize,  
        }  
    } else {  
        // 文件映射实现  
        if fd < 0 {  
            return -LinuxError::EBADF.code() as isize;  
        }  
          
        // 为文件映射在用户地址空间中分配地址  
        let start_addr = VirtAddr::from(0x20000000);  
          
        // 首先在地址空间中建立映射  
        match aspace.map_alloc(start_addr, aligned_length, mapping_flags, true) {  
            Ok(_) => {  
                // 映射建立成功后，读取文件内容到映射的内存中  
                let buffer_regions = aspace.translated_byte_buffer(start_addr, length);  
                  
                if let Some(regions) = buffer_regions {  
                    let mut total_read = 0;  
                      
                    for region in regions {  
                        if total_read >= length {  
                            break;  
                        }  
                          
                        let read_size = core::cmp::min(region.len(), length - total_read);  
                        let buffer = &mut region[..read_size];  
                          
                        // 尝试读取文件内容  
                        if let Ok(file_like) = get_file_like(fd) {  
                            match file_like.read(buffer) {  
                                Ok(bytes_read) => {  
                                    total_read += bytes_read;  
                                    // 如果读取的字节数少于缓冲区大小，清零剩余部分  
                                    if bytes_read < buffer.len() {  
                                        unsafe {  
                                            core::ptr::write_bytes(  
                                                buffer.as_mut_ptr().add(bytes_read),  
                                                0,  
                                                buffer.len() - bytes_read  
                                            );  
                                        }  
                                    }  
                                }  
                                Err(_) => {  
                                    // 读取失败，清零整个区域  
                                    unsafe {  
                                        core::ptr::write_bytes(buffer.as_mut_ptr(), 0, buffer.len());  
                                    }  
                                }  
                            }  
                        } else {  
                            // 无法获取文件对象，清零区域  
                            unsafe {  
                                core::ptr::write_bytes(buffer.as_mut_ptr(), 0, buffer.len());  
                            }  
                        }  
                    }  
                }  
                  
                return start_addr.as_usize() as isize;  
            }  
            Err(_) => return -LinuxError::ENOMEM.code() as isize,  
        }  
    }  
}

fn sys_openat(dfd: c_int, fname: *const c_char, flags: c_int, mode: api::ctypes::mode_t) -> isize {
    assert_eq!(dfd, AT_FDCWD);
    api::sys_open(fname, flags, mode) as isize
}

fn sys_close(fd: i32) -> isize {
    api::sys_close(fd) as isize
}

fn sys_read(fd: i32, buf: *mut c_void, count: usize) -> isize {
    api::sys_read(fd, buf, count)
}

fn sys_write(fd: i32, buf: *const c_void, count: usize) -> isize {
    api::sys_write(fd, buf, count)
}

fn sys_writev(fd: i32, iov: *const api::ctypes::iovec, iocnt: i32) -> isize {
    unsafe { api::sys_writev(fd, iov, iocnt) }
}

fn sys_set_tid_address(tid_ptd: *const i32) -> isize {
    let curr = current();
    curr.task_ext().set_clear_child_tid(tid_ptd as _);
    curr.id().as_u64() as isize
}

fn sys_ioctl(_fd: i32, _op: usize, _argp: *mut c_void) -> i32 {
    ax_println!("Ignore SYS_IOCTL");
    0
}
