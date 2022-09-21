#![allow(unused_variables)]
#![allow(dead_code)]
use crate::structs::*;
use std::fs::File;
use std::io;
use std::os::unix::prelude::FileExt;

impl BtrfsSuperblock {
    pub fn new() -> BtrfsSuperblock {
        unsafe { std::mem::zeroed() }
    }

    fn get_superblock(&mut self, file: &File) -> Result<(), std::io::Error> {
        let superblock_size = std::mem::size_of::<BtrfsSuperblock>();

        let bytes;
        unsafe {
            bytes = std::slice::from_raw_parts_mut(self as *mut _ as *mut u8, superblock_size);
        }

        file.read_exact_at(bytes, BTRFS_SUPERBLOCK_OFFSET)?;

        Ok(())
    }

    pub fn check_valid_superblock(&mut self, file: &File, debug: bool) -> io::Result<()> {
        self.get_superblock(&file)?;

        if self.magic != BTRFS_SUPERBLOCK_MAGIC {
            println!("Error reading the superblock {:?}", self.magic);
            std::process::exit(1);
        }
        if debug == true {
            println!(
                "sys_chunk_array_size: {}",
                self.sys_chunk_array_size as usize
            );
        }
        Ok(())
    }
}
