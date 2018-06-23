

use constants;


pub struct BlockMeta {
    line_mark: [bool; constants::LINE_SIZE],
    block_mark: bool,
}


impl BlockMeta {
    pub fn new_boxed() -> Box<BlockMeta> {
        Box::new(BlockMeta {
            line_mark: [false; constants::LINE_SIZE],
            block_mark: false,
        })
    }

    pub fn mark_line(&mut self, index: usize) {
        self.line_mark[index] = true;
    }

    pub fn mark_block(&mut self) {
        self.block_mark = true;
    }

    pub fn reset(&mut self) {
        for bit in self.line_mark.iter_mut() {
            *bit = false
        }
        self.block_mark = false;
    }

    pub fn iter<'it>(&'it self) -> impl Iterator<Item = &'it bool> {
        self.line_mark.iter()
    }
}
