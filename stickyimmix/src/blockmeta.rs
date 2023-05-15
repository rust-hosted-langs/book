use crate::constants;

/// Block marking metadata. This metadata is stored at the end of a Block.
// ANCHOR: DefBlockMeta
pub struct BlockMeta {
    lines: *mut u8,
}
// ANCHOR_END: DefBlockMeta

impl BlockMeta {
    /// Heap allocate a metadata instance so that it doesn't move so we can store pointers
    /// to it.
    pub fn new(block_ptr: *const u8) -> BlockMeta {
        let mut meta = BlockMeta {
            lines: unsafe { block_ptr.add(constants::LINE_MARK_START) as *mut u8 },
        };

        meta.reset();

        meta
    }

    unsafe fn as_block_mark(&mut self) -> &mut u8 {
        // Use the last byte of the block because no object will occupy the line
        // associated with this: it's the mark bits.
        &mut *self.lines.add(constants::LINE_COUNT - 1)
    }

    unsafe fn as_line_mark(&mut self, line: usize) -> &mut u8 {
        &mut *self.lines.add(line)
    }

    /// Mark the indexed line
    pub fn mark_line(&mut self, index: usize) {
        unsafe { *self.as_line_mark(index) = 1 };
    }

    /// Indicate the entire block as marked
    pub fn mark_block(&mut self) {
        unsafe { *self.as_block_mark() = 1 }
    }

    /// Reset all mark flags to unmarked.
    pub fn reset(&mut self) {
        unsafe {
            for i in 0..constants::LINE_COUNT {
                *self.lines.add(i) = 0;
            }
        }
    }

    /// Return an iterator over all the line mark flags
    //pub fn line_iter(&self) -> impl Iterator<Item = &'_ bool> {
    //    self.line_mark.iter()
    //}

    // ANCHOR: DefFindNextHole
    /// When it comes to finding allocatable holes, we bump-allocate downward.
    pub fn find_next_available_hole(
        &self,
        starting_at: usize,
        alloc_size: usize,
    ) -> Option<(usize, usize)> {
        // The count of consecutive avaliable holes. Must take into account a conservatively marked
        // hole at the beginning of the sequence.
        let mut count = 0;
        let starting_line = starting_at / constants::LINE_SIZE;
        let lines_required = (alloc_size + constants::LINE_SIZE - 1) / constants::LINE_SIZE;
        // Counting down from the given search start index
        let mut end = starting_line;

        for index in (0..starting_line).rev() {
            let marked = unsafe { *self.lines.add(index) };

            if marked == 0 {
                // count unmarked lines
                count += 1;

                if index == 0 && count >= lines_required {
                    let limit = index * constants::LINE_SIZE;
                    let cursor = end * constants::LINE_SIZE;
                    return Some((cursor, limit));
                }
            } else {
                // This block is marked
                if count > lines_required {
                    // But at least 2 previous blocks were not marked. Return the hole, considering the
                    // immediately preceding block as conservatively marked
                    let limit = (index + 2) * constants::LINE_SIZE;
                    let cursor = end * constants::LINE_SIZE;
                    return Some((cursor, limit));
                }

                // If this line is marked and we didn't return a new cursor/limit pair by now,
                // reset the hole search state
                count = 0;
                end = index;
            }
        }

        None
    }
    // ANCHOR_END: DefFindNextHole
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::blockalloc::Block;

    #[test]
    fn test_find_next_hole() {
        // A set of marked lines with a couple holes.
        // The first hole should be seen as conservatively marked.
        // The second hole should be the one selected.
        let block = Block::new(constants::BLOCK_SIZE).unwrap();
        let mut meta = BlockMeta::new(block.as_ptr());

        meta.mark_line(0);
        meta.mark_line(1);
        meta.mark_line(2);
        meta.mark_line(4);
        meta.mark_line(10);

        // line 5 should be conservatively marked
        let expect = Some((10 * constants::LINE_SIZE, 6 * constants::LINE_SIZE));

        let got = meta.find_next_available_hole(10 * constants::LINE_SIZE, constants::LINE_SIZE);

        println!("test_find_next_hole got {:?} expected {:?}", got, expect);

        assert!(got == expect);
    }

    #[test]
    fn test_find_next_hole_at_line_zero() {
        // Should find the hole starting at the beginning of the block
        let block = Block::new(constants::BLOCK_SIZE).unwrap();
        let mut meta = BlockMeta::new(block.as_ptr());

        meta.mark_line(3);
        meta.mark_line(4);
        meta.mark_line(5);

        let expect = Some((3 * constants::LINE_SIZE, 0));

        let got = meta.find_next_available_hole(3 * constants::LINE_SIZE, constants::LINE_SIZE);

        println!(
            "test_find_next_hole_at_line_zero got {:?} expected {:?}",
            got, expect
        );

        assert!(got == expect);
    }

    #[test]
    fn test_find_next_hole_at_block_end() {
        // The first half of the block is marked.
        // The second half of the block should be identified as a hole.
        let block = Block::new(constants::BLOCK_SIZE).unwrap();
        let mut meta = BlockMeta::new(block.as_ptr());

        let halfway = constants::LINE_COUNT / 2;

        for i in halfway..constants::LINE_COUNT {
            meta.mark_line(i);
        }

        // because halfway line should be conservatively marked
        let expect = Some((halfway * constants::LINE_SIZE, 0));

        let got = meta.find_next_available_hole(constants::BLOCK_CAPACITY, constants::LINE_SIZE);

        println!(
            "test_find_next_hole_at_block_end got {:?} expected {:?}",
            got, expect
        );

        assert!(got == expect);
    }

    #[test]
    fn test_find_hole_all_conservatively_marked() {
        // Every other line is marked.
        // No hole should be found.
        let block = Block::new(constants::BLOCK_SIZE).unwrap();
        let mut meta = BlockMeta::new(block.as_ptr());

        for i in 0..constants::LINE_COUNT {
            if i % 2 == 0 {
                // there is no stable step function for range
                meta.mark_line(i);
            }
        }

        let got = meta.find_next_available_hole(constants::BLOCK_CAPACITY, constants::LINE_SIZE);

        println!(
            "test_find_hole_all_conservatively_marked got {:?} expected None",
            got
        );

        assert!(got == None);
    }

    #[test]
    fn test_find_entire_block() {
        // No marked lines. Entire block is available.
        let block = Block::new(constants::BLOCK_SIZE).unwrap();
        let meta = BlockMeta::new(block.as_ptr());

        let expect = Some((constants::BLOCK_CAPACITY, 0));
        let got = meta.find_next_available_hole(constants::BLOCK_CAPACITY, constants::LINE_SIZE);

        println!("test_find_entire_block got {:?} expected {:?}", got, expect);

        assert!(got == expect);
    }
}
