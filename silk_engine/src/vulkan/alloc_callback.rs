//! Function overrides for allocating Vulkan objects (CPU only).
//! callbacks are used for logging allocations for debugging purposes

use crate::util::mem::Mem;
use ash::vk;
use std::alloc::{Layout, alloc, dealloc, realloc};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

trait AllocHandlerImpl: Send + Sync + Clone {
    fn on_alloc(
        &mut self,
        ptr: *mut c_void,
        size: usize,
        alignment: usize,
        scope: vk::SystemAllocationScope,
    );
    fn on_realloc(
        &mut self,
        ptr: *mut c_void,
        new_ptr: *mut c_void,
        old_size: usize,
        new_size: usize,
        alignment: usize,
        scope: vk::SystemAllocationScope,
    );
    fn on_free(&mut self, ptr: *mut c_void, size: usize, scope: vk::SystemAllocationScope);
    fn on_internal_alloc(
        &mut self,
        size: usize,
        alloc_type: vk::InternalAllocationType, // always "EXECUTABLE", ignored
        scope: vk::SystemAllocationScope,
    );
    fn on_internal_free(
        &mut self,
        size: usize,
        alloc_type: vk::InternalAllocationType, // always "EXECUTABLE", ignored
        scope: vk::SystemAllocationScope,
    );
}

#[derive(Clone)]
pub enum AllocHandler {
    Console { min_print_size: usize },
    NoOp,
}

impl AllocHandler {
    const ALLOC_COL: &str = "\x1b[38;2;241;76;76m";
    const FREE_COL: &str = "\x1b[38;2;35;209;139m";
    const ALIGN_COL: &str = "\x1b[38;41;184;219m";
}

impl AllocHandlerImpl for AllocHandler {
    fn on_alloc(
        &mut self,
        ptr: *mut c_void,
        size: usize,
        alignment: usize,
        scope: vk::SystemAllocationScope,
    ) {
        match self {
            AllocHandler::Console { min_print_size } => {
                if size < *min_print_size {
                    return;
                }
                println!(
                    "[VkAlloc] \x1b[2m{ptr:p}\x1b[0m {}{}\x1b[0m {}a{alignment}\x1b[0m {scope:?}",
                    Self::ALLOC_COL,
                    Mem::b(size),
                    Self::ALIGN_COL
                );
            }
            AllocHandler::NoOp => {}
        }
    }

    fn on_realloc(
        &mut self,
        ptr: *mut c_void,
        new_ptr: *mut c_void,
        old_size: usize,
        new_size: usize,
        alignment: usize,
        scope: vk::SystemAllocationScope,
    ) {
        match self {
            AllocHandler::Console { min_print_size } => {
                if new_size < *min_print_size {
                    return;
                }
                println!(
                    "[VkAlloc] (\x1b[2m{ptr:p}\x1b[0m {}{}\x1b[0m) => (\x1b[2m{new_ptr:p}\x1b[0m {}{}\x1b[0m) {}a{alignment}\x1b[0m {scope:?}",
                    Self::FREE_COL,
                    Mem::b(old_size),
                    Self::ALLOC_COL,
                    Mem::b(new_size),
                    Self::ALIGN_COL
                );
            }
            AllocHandler::NoOp => {}
        }
    }

    fn on_free(&mut self, ptr: *mut c_void, size: usize, scope: vk::SystemAllocationScope) {
        match self {
            AllocHandler::Console { min_print_size } => {
                if size < *min_print_size {
                    return;
                }
                println!(
                    "[VkAlloc] \x1b[2m{ptr:p}\x1b[0m {}{}\x1b[0m {scope:?}",
                    Self::FREE_COL,
                    Mem::b(size)
                );
            }
            AllocHandler::NoOp => {}
        }
    }

    fn on_internal_alloc(
        &mut self,
        size: usize,
        _alloc_type: vk::InternalAllocationType,
        scope: vk::SystemAllocationScope,
    ) {
        match self {
            AllocHandler::Console { min_print_size } => {
                if size < *min_print_size {
                    return;
                }
                println!(
                    "[VkAlloc] {}{}\x1b[0m {scope:?}",
                    Self::ALLOC_COL,
                    Mem::b(size)
                );
            }
            AllocHandler::NoOp => {}
        }
    }

    fn on_internal_free(
        &mut self,
        size: usize,
        _alloc_type: vk::InternalAllocationType,
        scope: vk::SystemAllocationScope,
    ) {
        match self {
            AllocHandler::Console { min_print_size } => {
                if size < *min_print_size {
                    return;
                }
                println!(
                    "[VkAlloc] {}{}\x1b[0m {scope:?}",
                    Self::FREE_COL,
                    Mem::b(size)
                );
            }
            AllocHandler::NoOp => {}
        }
    }
}


/// Tracks allocations + size, used for correctly freeing allocated vulkan objects. also logs allocations
pub(crate) struct AllocManager {
    // ptr: (layout, scope)
    allocs: HashMap<usize, (core::alloc::Layout, vk::SystemAllocationScope)>,
    logger: AllocHandler,
}

impl AllocManager {
    pub(crate) fn new(logger: AllocHandler) -> Self {
        Self {
            allocs: Default::default(),
            logger,
        }
    }

    fn alloc(
        &mut self,
        size: usize,
        alignment: usize,
        scope: vk::SystemAllocationScope,
    ) -> *mut c_void {
        let layout = Layout::from_size_align(size, alignment).unwrap();
        let ptr = unsafe { alloc(layout) as *mut c_void };
        self.allocs.insert(ptr as usize, (layout, scope));
        self.logger
            .on_alloc(ptr, layout.size(), layout.align(), scope);
        ptr
    }

    fn realloc(
        &mut self,
        ptr: *mut c_void,
        size: usize,
        alignment: usize,
        scope: vk::SystemAllocationScope,
    ) -> *mut c_void {
        if ptr.is_null() {
            return self.alloc(size, alignment, scope);
        }

        let (old_layout, _old_scope) = self.allocs.remove(&(ptr as usize)).unwrap();
        let new_ptr = unsafe { realloc(ptr as *mut u8, old_layout, size) as *mut c_void };
        if !new_ptr.is_null() {
            let new_layout = Layout::from_size_align(size, alignment).unwrap();
            self.allocs.insert(new_ptr as usize, (new_layout, scope));
            self.logger.on_realloc(
                ptr,
                new_ptr,
                old_layout.size(),
                new_layout.size(),
                new_layout.align(),
                scope,
            );
        }
        new_ptr
    }

    fn dealloc(&mut self, ptr: *mut c_void) {
        if ptr.is_null() {
            return;
        }
        let (layout, scope) = self.allocs.remove(&(ptr as usize)).unwrap();
        unsafe { dealloc(ptr as *mut u8, layout) };
        self.logger.on_free(ptr, layout.size(), scope);
    }

    fn alloc_internal(
        &mut self,
        size: usize,
        alloc_type: vk::InternalAllocationType,
        scope: vk::SystemAllocationScope,
    ) {
        self.logger.on_internal_alloc(size, alloc_type, scope);
    }

    fn free_internal(
        &mut self,
        size: usize,
        alloc_type: vk::InternalAllocationType,
        scope: vk::SystemAllocationScope,
    ) {
        self.logger.on_internal_free(size, alloc_type, scope);
    }

    unsafe extern "system" fn alloc_fn(
        user_data: *mut c_void,
        size: usize,
        alignment: usize,
        scope: vk::SystemAllocationScope,
    ) -> *mut c_void {
        let mutex = unsafe { &*(user_data as *const Mutex<Self>) };
        mutex.lock().unwrap().alloc(size, alignment, scope)
    }

    unsafe extern "system" fn realloc_fn(
        user_data: *mut c_void,
        ptr: *mut c_void,
        size: usize,
        alignment: usize,
        scope: vk::SystemAllocationScope,
    ) -> *mut c_void {
        let mutex = unsafe { &*(user_data as *const Mutex<Self>) };
        mutex.lock().unwrap().realloc(ptr, size, alignment, scope)
    }

    unsafe extern "system" fn free_fn(user_data: *mut c_void, ptr: *mut c_void) {
        let mutex = unsafe { &*(user_data as *const Mutex<Self>) };
        mutex.lock().unwrap().dealloc(ptr);
    }

    unsafe extern "system" fn internal_alloc_fn(
        user_data: *mut c_void,
        size: usize,
        alloc_type: vk::InternalAllocationType,
        scope: vk::SystemAllocationScope,
    ) {
        let mutex = unsafe { &*(user_data as *const Mutex<Self>) };
        mutex
            .lock()
            .unwrap()
            .alloc_internal(size, alloc_type, scope);
    }

    unsafe extern "system" fn internal_free_fn(
        user_data: *mut c_void,
        size: usize,
        alloc_type: vk::InternalAllocationType,
        scope: vk::SystemAllocationScope,
    ) {
        let mutex = unsafe { &*(user_data as *const Mutex<Self>) };
        mutex.lock().unwrap().free_internal(size, alloc_type, scope);
    }

    pub(crate) fn allocation_callbacks(
        alloc_manager: Arc<Mutex<Self>>,
    ) -> Option<vk::AllocationCallbacks<'static>> {
        if cfg!(debug_assertions) {
            // theoritically leaks memory, but does not matter
            let user_data = Arc::into_raw(alloc_manager.clone()) as *mut c_void;
            Some(
                vk::AllocationCallbacks::default()
                    .pfn_allocation(Some(Self::alloc_fn))
                    .pfn_reallocation(Some(Self::realloc_fn))
                    .pfn_free(Some(Self::free_fn))
                    .pfn_internal_allocation(Some(Self::internal_alloc_fn))
                    .pfn_internal_free(Some(Self::internal_free_fn))
                    .user_data(user_data),
            )
        } else {
            None
        }
    }
}
