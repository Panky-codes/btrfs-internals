#![allow(unused_variables)]
#![allow(dead_code)]
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::os::unix::prelude::FileExt;

use anyhow::{bail, Ok, Result};
use btrfs_internals::chunk_tree_cache::ChunkTree;
use btrfs_internals::ctree::{parse_sys_chunk_array, read_chunk_tree_root, walk_chunk_root_tree};
use btrfs_internals::structs::{
    BtrfsDirItem, BtrfsHeader, BtrfsInodeRef, BtrfsItem, BtrfsKey, BtrfsKeyPtr, BtrfsRootItem,
    BtrfsSuperblock, BTRFS_DIR_ITEM_KEY, BTRFS_FS_TREE_OBJECTID, BTRFS_FT_REG_FILE,
    BTRFS_INODE_REF_KEY, BTRFS_ROOT_ITEM_KEY,
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

#[derive(Debug)]
struct InodeRefT {
    key: BtrfsKey,
    name: String,
}

fn read_inode_ref_items(
    file: &File,
    fs_tree: &Vec<u8>,
    cache: &ChunkTree,
    nodesize: u32,
    inode_ref_cache: &mut HashMap<u64, InodeRefT>,
) -> Result<()> {
    let header = unsafe { &*(fs_tree.as_ptr() as *const BtrfsHeader) };

    // At the leaf
    if header.level == 0 {
        for i in 0..header.nritems as usize {
            let item = unsafe {
                &*((fs_tree.as_ptr() as usize
                    + std::mem::size_of::<BtrfsHeader>()
                    + (i * std::mem::size_of::<BtrfsItem>()))
                    as *const BtrfsItem)
            };

            if item.key.ty != BTRFS_INODE_REF_KEY {
                continue;
            }

            let inode_ref = unsafe {
                &*((fs_tree.as_ptr() as usize
                    + std::mem::size_of::<BtrfsHeader>()
                    + item.offset as usize) as *const BtrfsInodeRef)
            };

            let inode_name_slice = unsafe {
                std::slice::from_raw_parts(
                    (inode_ref as *const _ as *const u8).add(std::mem::size_of::<BtrfsInodeRef>()),
                    inode_ref.name_len as usize,
                )
            };
            let name = std::str::from_utf8(inode_name_slice)?.to_string();

            inode_ref_cache.insert(
                item.key.objectid,
                InodeRefT {
                    key: item.key,
                    name: name,
                },
            );
        }
    } else {
        println!("Node");
        for i in 0..header.nritems as usize {
            let keyptr = unsafe {
                &*((fs_tree.as_ptr() as usize
                    + std::mem::size_of::<BtrfsHeader>()
                    + (i * std::mem::size_of::<BtrfsKeyPtr>()))
                    as *const BtrfsKeyPtr)
            };

            let physical_offset = cache.offset(keyptr.blockptr).expect("error getting offset");

            let mut node = vec![0; nodesize as usize];
            file.read_exact_at(&mut node, physical_offset)?;
            read_inode_ref_items(file, fs_tree, cache, nodesize, inode_ref_cache)?;
        }
    }
    Ok(())
}

fn print_file_path(
    file: &File,
    fs_tree: &Vec<u8>,
    cache: &ChunkTree,
    nodesize: u32,
    inode_ref_cache: &HashMap<u64, InodeRefT>,
) -> Result<()> {
    let header = unsafe { &*(fs_tree.as_ptr() as *const BtrfsHeader) };

    // At the leaf
    if header.level == 0 {
        for i in 0..header.nritems as usize {
            let item = unsafe {
                &*((fs_tree.as_ptr() as usize
                    + std::mem::size_of::<BtrfsHeader>()
                    + (i * std::mem::size_of::<BtrfsItem>()))
                    as *const BtrfsItem)
            };

            if item.key.ty != BTRFS_DIR_ITEM_KEY {
                continue;
            }

            let dir_item = unsafe {
                &*((fs_tree.as_ptr() as usize
                    + std::mem::size_of::<BtrfsHeader>()
                    + item.offset as usize) as *const BtrfsDirItem)
            };

            if dir_item.ty != BTRFS_FT_REG_FILE {
                continue;
            }

            let mut path = String::with_capacity(1);
            let mut curr_inode_nr = dir_item.location.objectid;
            loop {
                let inode_ref = inode_ref_cache
                    .get(&curr_inode_nr)
                    .expect("Couldn't find inode");

                let parent_inode_nr = inode_ref.key.offset;

                if curr_inode_nr == parent_inode_nr {
                    break;
                }
                path.insert_str(0, &format!("/{}" ,&inode_ref.name));
                // Traverse to the parent inode
                curr_inode_nr = parent_inode_nr;
            }
            println!("file: {}", path);
        }
    } else {
        println!("Node");
        for i in 0..header.nritems as usize {
            let keyptr = unsafe {
                &*((fs_tree.as_ptr() as usize
                    + std::mem::size_of::<BtrfsHeader>()
                    + (i * std::mem::size_of::<BtrfsKeyPtr>()))
                    as *const BtrfsKeyPtr)
            };

            let physical_offset = cache.offset(keyptr.blockptr).expect("error getting offset");

            let mut node = vec![0; nodesize as usize];
            file.read_exact_at(&mut node, physical_offset)?;
            print_file_path(file, fs_tree, cache, nodesize, inode_ref_cache)?;
        }
    }
    Ok(())
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

    walk_chunk_root_tree(
        &file,
        &chunk_tree_root,
        &mut chunktree_cache,
        superblock.node_size,
    )?;

    // fill fs tree
    // read root tree to find fs tree
    let root_tree = read_root_tree(&file, superblock.root, &chunktree_cache)?;

    let fs_tree_root =
        read_fs_tree_root(&file, &root_tree, &chunktree_cache, superblock.node_size)?;

    let mut inode_ref_map = HashMap::new();

    read_inode_ref_items(
        &file,
        &fs_tree_root,
        &chunktree_cache,
        superblock.node_size,
        &mut inode_ref_map,
    )?;

    print_file_path(
        &file,
        &fs_tree_root,
        &chunktree_cache,
        superblock.node_size,
        &inode_ref_map,
    )?;
    Ok(())
}
