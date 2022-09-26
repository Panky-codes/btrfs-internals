#![allow(unused_variables)]
#![allow(dead_code)]
use crate::chunk_tree_cache::{ChunkTree, ChunkTreeKey};
use crate::structs::*;
use std::fs::File;
use std::io;
use std::os::unix::prelude::FileExt;

pub fn parse_sys_chunk_array(sb: &BtrfsSuperblock) -> Result<ChunkTree, i32> {
    let mut offset: usize = 0;
    let key_size = std::mem::size_of::<BtrfsKey>();
    let chunk_size = std::mem::size_of::<BtrfsChunk>();
    let stripe_size = std::mem::size_of::<BtrfsStripe>();
    let mut chunk_tree = ChunkTree::new();

    if sb.sys_chunk_array_size == 0 {
        panic!("Invalid chunk array size!")
    }

    while offset < sb.sys_chunk_array_size as usize {
        let key = &sb.sys_chunk_array[offset..];
        let btrfskey = unsafe { &*(key.as_ptr() as *const BtrfsKey) };

        if btrfskey.ty != BTRFS_CHUNK_ITEM_KEY {
            panic!("Not chunk item");
        }
        offset += key_size;

        let chunk = &sb.sys_chunk_array[offset..];
        let btrfschunk = unsafe { *(chunk.as_ptr() as *const BtrfsChunk) };
        let num_stripes = btrfschunk.num_stripes as usize;

        if num_stripes == 0 {
            panic!("num stripes cannot be zero");
        }

        if num_stripes != 1 {
            println!("num stripes more than one! : {}", num_stripes);
        }

        let length = btrfschunk.length;

        offset += chunk_size + (num_stripes - 1) * stripe_size;

        chunk_tree.insert(
            ChunkTreeKey {
                start: btrfskey.offset,
                size: btrfschunk.length,
            },
            btrfschunk.stripe.offset,
        )?;
    }
    Ok(chunk_tree)
}

pub fn read_chunk_tree_root(
    file: &File,
    chunk_logical_root: u64,
    cache: &ChunkTree,
) -> io::Result<Vec<u8>> {
    let size = cache
        .find_logical(chunk_logical_root)
        .expect("Can't find the chunk")
        .0
        .size;

    let mut chunk_root = vec![0; size as usize];

    let physical_off = cache
        .offset(chunk_logical_root)
        .expect("Can't find the chunk");

    let root = file.read_exact_at(&mut chunk_root, physical_off)?;

    Ok(chunk_root)
}

pub fn walk_chunk_root_tree(
    file: &File,
    buf: &Vec<u8>,
    cache: &mut ChunkTree,
    nodesize: u32,
) -> io::Result<()> {
    let header = unsafe { &*(buf.as_ptr() as *const BtrfsHeader) };

    // At the leaf
    if header.level == 0 {
        for i in 0..header.nritems as usize {
            let item = unsafe {
                &*((buf.as_ptr() as usize
                    + std::mem::size_of::<BtrfsHeader>()
                    + (i * std::mem::size_of::<BtrfsItem>()))
                    as *const BtrfsItem)
            };

            if item.key.ty != BTRFS_CHUNK_ITEM_KEY {
                continue;
            }

            let chunk = unsafe {
                &*((buf.as_ptr() as usize
                    + std::mem::size_of::<BtrfsHeader>()
                    + item.offset as usize) as *const BtrfsChunk)
            };

            let off = item.key.offset;
            let len = chunk.length;
            let phy_off = chunk.stripe.offset;
            cache
                .insert(
                    ChunkTreeKey {
                        start: item.key.offset,
                        size: chunk.length,
                    },
                    chunk.stripe.offset,
                )
                .unwrap_or_else(|_| panic!("Error inserting cache"));
        }
    } else {
        println!("Node");
        for i in 0..header.nritems as usize {
            let keyptr = unsafe {
                &*((buf.as_ptr() as usize
                    + std::mem::size_of::<BtrfsHeader>()
                    + (i * std::mem::size_of::<BtrfsKeyPtr>()))
                    as *const BtrfsKeyPtr)
            };
            let physical_offset = cache.offset(keyptr.blockptr).expect("error getting offset");

            let mut node = vec![0; nodesize as usize];
            file.read_exact_at(&mut node, physical_offset)?;
            walk_chunk_root_tree(&file, &node, cache, nodesize)?;
        }
    }
    Ok(())
}
