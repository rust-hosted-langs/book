
extern crate blockalloc;

use blockalloc::{Block, BlockError};


struct Memory {
    blocks: Vec<Block>
}


impl Memory {
    pub fn new() -> Memory {
        Memory {
            blocks: Vec::new()
        }
    }

    pub fn alloc<T>(object: T) -> *mut T {

    }
}


#[cfg(test)]
mod tests {

    use Memory;


    #[test]
    fn test_memory() {
        let mem = Memory::new();
    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
