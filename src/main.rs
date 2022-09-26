#![allow(unused_variables)]
#![allow(dead_code)]
use std::env;
use std::fs::File;
use std::os::unix::prelude::FileExt;

use anyhow::{bail, Result};
use btrfs_internals::chunk_tree_cache::ChunkTree;
use btrfs_internals::ctree::{parse_sys_chunk_array, read_chunk_tree_root, walk_chunk_root_tree};
use btrfs_internals::structs::{
    BtrfsHeader, BtrfsItem, BtrfsRootItem, BtrfsSuperblock, BTRFS_FS_TREE_OBJECTID,
    BTRFS_ROOT_ITEM_KEY, BtrfsChunk,
};

fn read_root_tree(file: &File, root_logical: u64, cache: &ChunkTree) -> Result<Vec<u8>> {
    let size = cache
        .find_logical(root_logical)
        .expect("Can't find the chunk")
        .0
        .size;

    let physical = cache
        .offset(root_logical)
        .expect("error finding physical offset");

    let mut buf = vec![0; size as usize];

    file.read_exact_at(&mut buf, physical)?;

    Ok(buf)
}

fn read_fs_tree_root(
    file: &File,
    root_tree: &Vec<u8>,
    cache: &ChunkTree,
    nodesize: u32,
) -> Result<Vec<u8>> {
    let header = unsafe { &*(root_tree.as_ptr() as *const BtrfsHeader) };
    let mut buf = vec![0; nodesize as usize];

    if header.level != 0 {
        bail!("Root tree should be a leaf");
    }

    for i in 0..header.nritems as usize {
        let item = unsafe {
            &*((root_tree.as_ptr() as usize
                + std::mem::size_of::<BtrfsHeader>()
                + (i * std::mem::size_of::<BtrfsItem>())) as *const BtrfsItem)
        };

        if item.key.ty == BTRFS_ROOT_ITEM_KEY && item.key.objectid == BTRFS_FS_TREE_OBJECTID {
            let fs_root_item = unsafe {
                &*((root_tree.as_ptr() as usize
                    + std::mem::size_of::<BtrfsHeader>()
                    + item.offset as usize) as *const BtrfsRootItem)
            };
            let physical = cache
                .offset(fs_root_item.bytenr)
                .expect("error finding the physical offset");

            file.read_exact_at(&mut buf, physical)?;
        }
    }
    Ok(buf)
}
fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("No arguments provided");
        println!("usage: btrfs-internals <image>");
        panic!();
    }
    let file = File::open(&args[1])?;
    let mut superblock = BtrfsSuperblock::new();

    superblock.check_valid_superblock(&file, false)?;
    // bootstrap chunk tree from superblock
    let mut chunktree_cache = parse_sys_chunk_array(&superblock).unwrap_or_else(|error| {
        panic!("Error parsing the sys chunk arr");
    });

    // fill chunk tree
    let chunk_tree_root = read_chunk_tree_root(&file, superblock.chunk_root, &chunktree_cache)?;

    walk_chunk_root_tree::<BtrfsChunk>(
        &file,
        &chunk_tree_root,
        &mut chunktree_cache,
        superblock.node_size,
    )?;

    // fill fs tree
    // read root tree to find fs tree
    let root_tree = read_root_tree(&file, superblock.root, &chunktree_cache)?;

    let fs_tree_root = read_fs_tree_root(&file, &root_tree, &chunktree_cache, superblock.node_size)?;

    Ok(())
}
