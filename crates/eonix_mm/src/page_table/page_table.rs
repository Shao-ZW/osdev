use super::{
    paging_mode::PageTableLevel,
    pte::{RawAttribute, TableAttribute},
    pte_iterator::{KernelIterator, UserIterator},
    PagingMode, PTE,
};
use crate::{
    address::{PAddr, VRange},
    page_table::PageTableIterator,
    paging::{GlobalPageAlloc, Page, PageAccess, PageAlloc, PageBlock},
};
use core::{marker::PhantomData, ptr::NonNull};

pub trait RawPageTable<'a>: 'a {
    type Entry: PTE + 'a;

    /// Return the entry at the given index.
    fn index(&self, index: u16) -> &'a Self::Entry;

    /// Return a mutable reference to the entry at the given index.
    fn index_mut(&mut self, index: u16) -> &'a mut Self::Entry;

    /// Get the page table pointed to by raw pointer `ptr`.
    unsafe fn from_ptr(ptr: NonNull<PageBlock>) -> Self;
}

pub struct PageTable<'a, M, A, X>
where
    M: PagingMode,
    M::Entry: 'a,
    A: PageAlloc,
    X: PageAccess,
{
    root_table_page: Page<A>,
    phantom: PhantomData<&'a (M, X)>,
}

impl<'a, M, A, X> PageTable<'a, M, A, X>
where
    M: PagingMode,
    M::Entry: 'a,
    A: PageAlloc,
    X: PageAccess,
{
    pub fn new_in<A1: PageAlloc>(kernel_root_table_page: &Page<A1>, alloc: A) -> Self {
        let new_root_table_page = Page::alloc_in(alloc);
        let new_table_data = X::get_ptr_for_page(&new_root_table_page);
        let kernel_table_data = X::get_ptr_for_page(kernel_root_table_page);

        unsafe {
            // SAFETY: `new_table_data` and `kernel_table_data` are both valid pointers
            //         to **different** page tables.
            new_table_data.copy_from_nonoverlapping(kernel_table_data, 1);
        }

        let mut root_page_table = unsafe {
            // SAFETY: `page_table_ptr` is a valid pointer to a page table.
            M::RawTable::from_ptr(new_table_data)
        };

        let level0 = M::LEVELS[0];
        for idx in 0..=level0.max_index() / 2 {
            // We consider the first half of the page table as user space.
            // Clear all (potential) user space mappings.
            root_page_table.index_mut(idx).take();
        }

        Self {
            root_table_page: new_root_table_page,
            phantom: PhantomData,
        }
    }

    pub fn addr(&self) -> PAddr {
        self.root_table_page.start()
    }

    pub fn iter_user(&self, range: VRange) -> impl Iterator<Item = &mut M::Entry> {
        let alloc = self.root_table_page.allocator();
        let page_table_ptr = X::get_ptr_for_page(&self.root_table_page);
        let root_page_table = unsafe {
            // SAFETY: `page_table_ptr` is a valid pointer to a page table.
            M::RawTable::from_ptr(page_table_ptr)
        };

        PageTableIterator::<M, A, X, UserIterator>::new(root_page_table, range, alloc.clone())
    }

    pub fn iter_kernel(&self, range: VRange) -> impl Iterator<Item = &mut M::Entry> {
        Self::iter_kernel_levels(self, range, M::LEVELS)
    }

    /// Iterates over the kernel space entries in the page table for the specified levels.
    ///
    /// # Parameters
    /// - `range`: The virtual address range to iterate over.
    /// - `levels`: A slice of `PageTableLevel` that specifies which levels of the page table
    ///   should be included in the iteration. Each level corresponds to a level in the page
    ///   table hierarchy, and the iterator will traverse entries at these levels.
    ///
    /// # Returns
    /// An iterator over mutable references to the page table entries (`M::Entry`) within the
    /// specified range and levels.
    ///
    /// # Example
    /// ```
    /// let range = VRange::new(0x1234000, 0x1300000);
    /// let levels = &M::LEVELS[..2];
    /// for pte in page_table.iter_kernel_levels(range, levels) {
    ///     // Process each entry
    /// }
    /// ```
    pub fn iter_kernel_levels(
        &self,
        range: VRange,
        levels: &'static [PageTableLevel],
    ) -> impl Iterator<Item = &mut M::Entry> {
        let alloc = self.root_table_page.allocator();
        let page_table_ptr = X::get_ptr_for_page(&self.root_table_page);
        let root_page_table = unsafe {
            // SAFETY: `page_table_ptr` is a valid pointer to a page table.
            M::RawTable::from_ptr(page_table_ptr)
        };

        PageTableIterator::<M, A, X, KernelIterator>::with_levels(
            root_page_table,
            range,
            alloc.clone(),
            levels,
        )
    }

    fn drop_page_table_recursive(page_table: &Page<A>, levels: &[PageTableLevel]) {
        let [level, remaining_levels @ ..] = levels else { return };

        let alloc = page_table.allocator();

        let page_table_ptr = X::get_ptr_for_page(page_table);
        let mut page_table = unsafe {
            // SAFETY: `page_table_ptr` is a valid pointer to a page table.
            M::RawTable::from_ptr(page_table_ptr)
        };

        for pte in (0..=level.max_index()).map(|i| page_table.index_mut(i)) {
            let (pfn, attr) = pte.take();
            let Some(attr) = attr.as_table_attr() else {
                continue;
            };

            if !attr.contains(TableAttribute::PRESENT | TableAttribute::USER) {
                continue;
            }

            let page_table = unsafe {
                // SAFETY: We got the pfn from a valid page table entry, so it should be valid.
                Page::from_raw_in(pfn, alloc.clone())
            };

            Self::drop_page_table_recursive(&page_table, remaining_levels);
        }
    }
}

impl<'a, M, A, X> PageTable<'a, M, A, X>
where
    M: PagingMode,
    M::Entry: 'a,
    A: GlobalPageAlloc,
    X: PageAccess,
{
    pub fn new<A1: PageAlloc>(kernel_root_table_page: &Page<A1>) -> Self {
        Self::new_in(kernel_root_table_page, A::global())
    }
}

impl<'a, M, A, X> Drop for PageTable<'a, M, A, X>
where
    M: PagingMode,
    M::Entry: 'a,
    A: PageAlloc,
    X: PageAccess,
{
    fn drop(&mut self) {
        Self::drop_page_table_recursive(&self.root_table_page, M::LEVELS);
    }
}
