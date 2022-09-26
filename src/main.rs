#![allow(unused_variables)]
#![allow(dead_code)]
use std::fs::File;
use std::io;
use std::{env};

use btrfs_internals::ctree::{parse_sys_chunk_array, read_chunk_tree_root, walk_chunk_root_tree};
use btrfs_internals::{
    structs::{BtrfsSuperblock},
};


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
    let mut chunktree_cache = parse_sys_chunk_array(&superblock).unwrap_or_else(|error| {
        panic!("Error parsing the sys chunk arr");
    });

    let chunk_tree_root = read_chunk_tree_root(&file, superblock.chunk_root, &chunktree_cache)?;

    walk_chunk_root_tree(
        &file,
        &chunk_tree_root,
        &mut chunktree_cache,
        superblock.node_size,
    )?;
    println!("{}", chunktree_cache);

    Ok(())
}
