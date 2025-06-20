use super::FromSyscallArg;
use crate::kernel::constants::{EINVAL, ENOMEM};
use crate::{
    kernel::{
        constants::{UserMmapFlags, UserMmapProtocol},
        mem::{Mapping, Permission},
    },
    prelude::*,
};
use align_ext::AlignExt;
use eonix_mm::address::{Addr as _, AddrOps as _, VAddr};
use eonix_mm::paging::PAGE_SIZE;
use eonix_runtime::task::Task;

impl FromSyscallArg for UserMmapProtocol {
    fn from_arg(value: usize) -> UserMmapProtocol {
        UserMmapProtocol::from_bits_truncate(value as u32)
    }
}

impl FromSyscallArg for UserMmapFlags {
    fn from_arg(value: usize) -> UserMmapFlags {
        UserMmapFlags::from_bits_truncate(value as u32)
    }
}

/// Check whether we are doing an implemented function.
/// If `condition` is false, return `Err(err)`.
fn check_impl(condition: bool, err: u32) -> KResult<()> {
    if !condition {
        Err(err)
    } else {
        Ok(())
    }
}

#[eonix_macros::define_syscall(0xc0)]
fn mmap_pgoff(
    addr: usize,
    len: usize,
    prot: UserMmapProtocol,
    flags: UserMmapFlags,
    fd: u32,
    pgoffset: usize,
) -> KResult<usize> {
    let addr = VAddr::from(addr);
    if !addr.is_page_aligned() || len == 0 {
        return Err(EINVAL);
    }

    let len = len.align_up(PAGE_SIZE);
    check_impl(flags.contains(UserMmapFlags::MAP_ANONYMOUS), ENOMEM)?;
    check_impl(flags.contains(UserMmapFlags::MAP_PRIVATE), EINVAL)?;
    if fd != u32::MAX || pgoffset != 0 {
        return Err(EINVAL);
    }

    let mm_list = &thread.process.mm_list;

    // TODO!!!: If we are doing mmap's in 32-bit mode, we should check whether
    //          `addr` is above user reachable memory.
    let addr = if flags.contains(UserMmapFlags::MAP_FIXED) {
        if prot.is_empty() {
            Task::block_on(mm_list.protect(
                addr,
                len,
                Permission {
                    read: prot.contains(UserMmapProtocol::PROT_READ),
                    write: prot.contains(UserMmapProtocol::PROT_WRITE),
                    execute: prot.contains(UserMmapProtocol::PROT_EXEC),
                },
            ))
            .map(|_| addr)
        } else {
            mm_list.mmap_fixed(
                addr,
                len,
                Mapping::Anonymous,
                Permission {
                    read: prot.contains(UserMmapProtocol::PROT_READ),
                    write: prot.contains(UserMmapProtocol::PROT_WRITE),
                    execute: prot.contains(UserMmapProtocol::PROT_EXEC),
                },
            )
        }
    } else {
        mm_list.mmap_hint(
            addr,
            len,
            Mapping::Anonymous,
            Permission {
                read: prot.contains(UserMmapProtocol::PROT_READ),
                write: prot.contains(UserMmapProtocol::PROT_WRITE),
                execute: prot.contains(UserMmapProtocol::PROT_EXEC),
            },
        )
    };

    addr.map(|addr| addr.addr())
}

#[eonix_macros::define_syscall(0x5b)]
fn munmap(addr: usize, len: usize) -> KResult<usize> {
    let addr = VAddr::from(addr);
    if !addr.is_page_aligned() || len == 0 {
        return Err(EINVAL);
    }

    let len = len.align_up(PAGE_SIZE);
    Task::block_on(thread.process.mm_list.unmap(addr, len)).map(|_| 0)
}

#[eonix_macros::define_syscall(0x2d)]
fn brk(addr: usize) -> KResult<usize> {
    let vaddr = if addr == 0 { None } else { Some(VAddr::from(addr)) };
    Ok(thread.process.mm_list.set_break(vaddr).addr())
}

#[eonix_macros::define_syscall(0xdb)]
fn madvise(_addr: usize, _len: usize, _advice: u32) -> KResult<()> {
    Ok(())
}

#[eonix_macros::define_syscall(0x7d)]
fn mprotect(addr: usize, len: usize, prot: UserMmapProtocol) -> KResult<()> {
    let addr = VAddr::from(addr);
    if !addr.is_page_aligned() || len == 0 {
        return Err(EINVAL);
    }

    let len = len.align_up(PAGE_SIZE);

    Task::block_on(thread.process.mm_list.protect(
        addr,
        len,
        Permission {
            read: prot.contains(UserMmapProtocol::PROT_READ),
            write: prot.contains(UserMmapProtocol::PROT_WRITE),
            execute: prot.contains(UserMmapProtocol::PROT_EXEC),
        },
    ))
}

#[eonix_macros::define_syscall(0x177)]
fn membarrier(_cmd: usize, _flags: usize) -> KResult<()> {
    Ok(())
}

pub fn keep_alive() {}
