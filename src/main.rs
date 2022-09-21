#![allow(unused_variables)]
#![allow(dead_code)]
use std::env;
use std::fs::File;
use std::io;

use btrfs_internals::{
    chunk_tree::{ChunkTree, ChunkTreeKey},
    structs::{BtrfsChunk, BtrfsKey, BtrfsStripe, BtrfsSuperblock, BTRFS_CHUNK_ITEM_KEY},
};

fn parse_sys_chunk_array(sb: &BtrfsSuperblock) -> Result<ChunkTree, i32> {
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

// fn read_chunk_tree_root()
fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("No arguments provided");
        println!("usage: btrfs-internals <image>");
        panic!();
    }
    let file = File::open(&args[1])?;
    let mut superblock = BtrfsSuperblock::new();

    superblock.check_valid_superblock(&file, false)?;
    let parsed_chunktree = parse_sys_chunk_array(&superblock).unwrap_or_else(|error| {
        panic!("Error parsing the sys chunk arr");
    });

    Ok(())
}
