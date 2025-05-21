#![no_std]

use allocator::{BaseAllocator, ByteAllocator, PageAllocator};

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const SIZE: usize> {  
    // 内存区域的起始地址  
    start: usize,  
    // 内存区域的大小  
    size: usize,  
    // 字节分配的当前位置  
    bytes_pos: usize,  
    // 页面分配的当前位置  
    pages_pos: usize,  
    // 字节分配的计数  
    bytes_count: usize,  
}
impl<const SIZE: usize> EarlyAllocator<SIZE> {  
    pub const fn new() -> Self {  
        Self {  
            start: 0,  
            size: 0,  
            bytes_pos: 0,  
            pages_pos: 0,  
            bytes_count: 0,  
        }  
    }  
}


impl<const SIZE: usize> BaseAllocator for EarlyAllocator<SIZE> {  
    fn init(&mut self, start: usize, size: usize) {  
        self.start = start;  
        self.size = size;  
        self.bytes_pos = start;  
        self.pages_pos = start + size;  
        self.bytes_count = 0;  
    }  
  
    fn add_memory(&mut self, start: usize, size: usize) -> allocator::AllocResult {  
        // 简单的实现可以直接返回错误，因为早期分配器通常不支持添加内存  
        Err(allocator::AllocError::NoMemory)  
    }  
}
impl<const SIZE: usize> ByteAllocator for EarlyAllocator<SIZE> {  
    fn alloc(  
        &mut self,  
        layout: core::alloc::Layout,  
    ) -> allocator::AllocResult<core::ptr::NonNull<u8>> {  
        // 计算对齐后的地址  
        let align = layout.align();  
        let bytes_pos_aligned = (self.bytes_pos + align - 1) & !(align - 1);  
          
        // 计算分配后的新位置  
        let new_bytes_pos = bytes_pos_aligned + layout.size();  
          
        // 检查是否有足够的空间  
        if new_bytes_pos > self.pages_pos {  
            return Err(allocator::AllocError::NoMemory);  
        }  
          
        // 更新位置和计数  
        let ptr = bytes_pos_aligned;  
        self.bytes_pos = new_bytes_pos;  
        self.bytes_count += 1;  
          
        // 返回分配的内存指针  
        Ok(unsafe { core::ptr::NonNull::new_unchecked(ptr as *mut u8) })  
    }  
  
    fn dealloc(&mut self, _pos: core::ptr::NonNull<u8>, _layout: core::alloc::Layout) {  
        // 减少计数，如果计数为0，重置bytes_pos  
        self.bytes_count -= 1;  
        if self.bytes_count == 0 {  
            self.bytes_pos = self.start;  
        }  
    }  
  
    fn total_bytes(&self) -> usize {  
        self.size  
    }  
  
    fn used_bytes(&self) -> usize {  
        self.bytes_pos - self.start  
    }  
  
    fn available_bytes(&self) -> usize {  
        self.pages_pos - self.bytes_pos  
    }  
}

impl<const SIZE: usize> PageAllocator for EarlyAllocator<SIZE> {  
    const PAGE_SIZE: usize = SIZE;  
  
    fn alloc_pages(  
        &mut self,  
        num_pages: usize,  
        align_pow2: usize,  
    ) -> allocator::AllocResult<usize> {  
        // 计算需要的大小  
        let size = num_pages * Self::PAGE_SIZE;  
          
        // 计算对齐后的地址  
        let aligned_pos = (self.pages_pos - size) & !(align_pow2 - 1);  
          
        // 检查是否有足够的空间  
        if aligned_pos < self.bytes_pos {  
            return Err(allocator::AllocError::NoMemory);  
        }  
          
        // 更新位置  
        self.pages_pos = aligned_pos;  
          
        // 返回分配的内存地址  
        Ok(aligned_pos)  
    }  
  
    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {  
        // 页面分配区域不会被释放，所以这里不需要做任何事情  
    }  
  
    fn total_pages(&self) -> usize {  
        self.size / Self::PAGE_SIZE  
    }  
  
    fn used_pages(&self) -> usize {  
        (self.start + self.size - self.pages_pos) / Self::PAGE_SIZE  
    }  
  
    fn available_pages(&self) -> usize {  
        (self.pages_pos - self.bytes_pos) / Self::PAGE_SIZE  
    }  
}