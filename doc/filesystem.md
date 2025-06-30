# 文件系统

Eonix 内核的文件系统模块通过 `Inode`、`Dentry` 和 `DentryCache` 数据结构实现高效的文件管理、路径解析和缓存机制。它们共同构成了 Eonix 内核文件系统的核心数据结构，提供了高效的文件操作和路径解析能力。`Inode` 描述文件的元数据和操作接口，`Dentry` 连接路径和文件内容，而 `DentryCache` 则通过缓存机制大幅提升文件系统的性能。三者的协作确保了文件系统在复杂操作和高并发环境下的高效性和稳定性。

---

#### 1. Inode：文件和目录的核心描述

`Inode` 是 Eonix 文件系统中表示文件或目录的核心结构，提供了文件元数据的存储与访问。它通过 `InodeData` 和 `Inode` 接口实现，支持文件的基础操作。

**基本属性：**

- `ino`：文件的唯一标识符。
- `size` 和 `nlink`：表示文件大小和硬链接计数。
- `uid` 和 `gid`：文件的拥有者和组。
- `mode`：文件的权限和类型（如目录、常规文件等）。
- `atime`、`ctime` 和 `mtime`：文件的访问、创建和修改时间。
- `rwsem`：读写信号量，用于并发访问控制。

**功能接口：**

- 提供创建文件（`creat`）、创建目录（`mkdir`）、读写文件（`read` 和 `write`）等基本操作。
- 支持通过 `lookup` 方法查找子文件，`statx` 获取文件元数据，`truncate` 修改文件大小。
- 提供与设备相关的接口，如 `readlink` 解析符号链接，`mknod` 创建特殊文件。

**并发支持：**

- 使用原子操作和信号量确保多线程环境下的安全性。
- 通过 `Spin` 和 `RwSemaphore` 实现时间和读写操作的并发控制。

---

#### 2. Dentry：用于路径解析与文件缓存

`Dentry` 是文件路径与 `Inode` 的映射关系，主要用于路径解析和目录缓存。

**基本结构：**

- `name` 和 `parent`：当前路径组件名称及其父目录的 `Dentry`。
- `data`：指向文件或目录的 `Inode`。
- `hash`：通过路径计算的哈希值，用于高效查找。
- 链表指针（`prev` 和 `next`）：支持 `DentryCache` 中的快速访问。

**功能接口：**

- `find` 和 `open_recursive` 支持路径解析，递归查找目录结构。
- `save_data` 和 `get_inode` 关联或获取 `Inode`。
- 提供对文件的操作接口，包括 `mkdir`、`unlink`、`readlink` 和 `write`。
- 支持符号链接解析，通过 `resolve_directory` 实现递归解析。
- 提供 `is_directory` 和 `is_valid` 方法检查当前节点是否为目录及其是否有效。

---

#### 3. DentryCache：加速文件路径解析

`DentryCache` 是路径解析的高速缓存，通过维护 `Dentry` 的哈希表加速文件和目录的查找。

**缓存结构**方面，我们使用定长哈希表（`DCACHE`）存储不同哈希值对应的 `Dentry` 列表。提供静态根目录（`DROOT`），便于路径解析的起点管理。

**缓存一致性**方面，使用 RCU（Read-Copy-Update）机制维护多线程环境下的缓存一致性，确保高效的并发访问。

**核心操作：**

- `d_add`：将新的 `Dentry` 加入缓存。
- `d_find_fast` 和 `d_iter_for`：根据哈希值快速查找缓存中的 `Dentry`。
- `d_replace` 和 `d_remove`：支持缓存中节点的替换和移除操作。
- `d_try_revalidate`：通过父目录的 `lookup` 方法验证节点有效性。

---

### 4. Inode、Dentry 和 DentryCache 的协作

#### 路径解析

用户请求文件路径时，内核通过 `Dentry` 递归解析路径。若缓存命中，`DentryCache` 提供快速访问；若缓存未命中，调用父目录的 `lookup` 方法查找对应的 `Inode`。

#### 文件操作

文件操作首先通过 `Dentry` 获取 `Inode`，然后调用 `Inode` 提供的读写、创建等接口完成操作。在操作结束后，通过 `DentryCache` 缓存结果，加速后续访问。

#### 并发访问

- `Inode` 的读写信号量（`rwsem`）和 `Dentry` 的 RCU 机制共同保障文件系统的并发访问安全。

---

### 5. PageCache: 加速文件读写

`PageCache`模块旨在优化文件系统和块设备 I/O 性能，通过在内存中维护常用数据页的副本，减少对底层存储设备的直接访问。该模块的核心是 `PageCache` 结构体，它管理着一个由页帧号索引的 `BTreeMap`，其中存储着 `CachePage` 实例。每个 `CachePage` 对应一个物理内存页，并包含该页的有效数据大小和脏页标志位。

`CachePage` 实现了 `Buffer trait`，使其能够方便地进行数据的填充和读取。当需要从文件或块设备中读取数据时，`PageCache::read` 方法会首先检查请求的数据页是否已经在缓存中。如果命中，则直接从 `CachePage` 中复制数据。如果未命中，模块会通过 `PageCacheBackend trait` 抽象出的后端接口从底层设备读取数据，然后将数据填充到新的 `CachePage` 中并将其插入缓存。类似地，`PageCache::write` 方法处理数据写入，它会查找或创建相应的 `CachePage`，将新数据写入其中，并设置脏页标志。`PageCache::fsync` 方法负责将所有标记为脏的缓存页写回后端存储，确保数据持久化。此外，`resize` 方法支持文件或块设备的扩展和截断操作，它会相应地调整缓存中页的范围。

`PageCacheBackend trait` 是页面缓存与具体存储设备（如文件系统或块设备）之间的桥梁，它定义了 `read_page`、`write_page` 和 `size` 等基本操作，允许页面缓存以统一的方式与不同的后端交互。`PageCache` 在其生命周期结束时会自动调用 `fsync`，以确保所有修改的数据都能被写回磁盘，从而保障数据的一致性和完整性。通过这种设计，页面缓存模块在提供高性能数据访问的同时，也保持了良好的可扩展性和与底层存储的解耦。

``` rust
pub struct PageCache {
    pages: Mutex<BTreeMap<usize, CachePage>>,
    backend: Weak<dyn PageCacheBackend>,
}

// with this trait, "page cache" and "block cache" are unified,
// for fs, offset is file offset (floor algin to PAGE_SIZE)
// for blkdev, offset is block idx (floor align to PAGE_SIZE / BLK_SIZE)
pub trait PageCacheBackend {
    fn read_page(&self, page: &mut CachePage, offset: usize) -> KResult<usize>;

    fn write_page(&self, page: &CachePage, offset: usize) -> KResult<usize>;

    fn size(&self) -> usize;
}
```
