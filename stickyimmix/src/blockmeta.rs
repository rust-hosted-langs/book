

use constants;


pub struct BlockMeta {
    line_mark: [bool; constants::LINE_COUNT],
    block_mark: bool,
}


impl BlockMeta {
    pub fn new_boxed() -> Box<BlockMeta> {
        Box::new(BlockMeta {
            line_mark: [false; constants::LINE_COUNT],
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

    /// Given a byte index into a block (the `starting_at` parameter) find the next available
    /// hole in which bump allocation can occur, or `None` if no hole can be found in this
    /// block.
    /// Takes into account conservative marking of the first unmarked line in a hole.
    pub fn find_next_available_hole(&self, starting_at: usize) -> Option<(usize, usize)> {
        let mut count = 0;
        let mut start: Option<usize> = None;
        let mut stop: usize = 0;

        let starting_line = starting_at * constants::LINE_SIZE;

        for (index, marked) in self.line_mark[starting_line..].iter().enumerate() {

            let abs_index = starting_at + index;

            // count unmarked lines
            if !*marked {
                count += 1;

                // record the first hole index
                if let None = start {
                    start = Some(abs_index);
                }

                // stop is the end of this line
                stop = abs_index + 1;

                // if this is the first hole (and not the zeroth line), consider it
                // conservatively marked and get the next line as the start if it's
                // unmarked
                if abs_index > 0 && count == 1 {
                    start = None;
                }
            }

            // if reached a marked line or the end of the block, see if we have
            // a valid hole to work with
            if *marked || stop >= constants::LINE_COUNT {

                if let Some(start) = start {
                    let cursor = start * constants::LINE_SIZE;
                    let limit = stop * constants::LINE_SIZE;

                    return Some((cursor, limit))
                }
            }

            // if this line is marked and we didn't return a new cursor/limit pair by now,
            // reset the hole state
            if *marked {
                count = 0;
                start = None;
            }
         }

        None
    }
}


#[cfg(test)]
mod tests {

    use super::*;


    #[test]
    fn test_find_next_hole() {
        let mut meta = BlockMeta::new_boxed();

        meta.mark_line(0);
        meta.mark_line(1);
        meta.mark_line(2);
        meta.mark_line(4);
        meta.mark_line(10);

        let expect = Some((6 * constants::LINE_SIZE, 10 * constants::LINE_SIZE));

        let got = meta.find_next_available_hole(0);

        println!("test_find_next_hole got {:?} expected {:?}", got, expect);

        assert!(got == expect);
    }

    #[test]
    fn test_find_next_hole_at_line_zero() {
        let mut meta = BlockMeta::new_boxed();

        meta.mark_line(3);
        meta.mark_line(4);
        meta.mark_line(5);

        let expect = Some((0, 3 * constants::LINE_SIZE));

        let got = meta.find_next_available_hole(0);

        println!("test_find_next_hole_at_line_zero got {:?} expected {:?}", got, expect);

        assert!(got == expect);
    }

    #[test]
    fn test_find_next_hole_at_block_end() {
        let mut meta = BlockMeta::new_boxed();

        let halfway = constants::LINE_COUNT / 2;

        for i in 0..halfway {
            meta.mark_line(i);
        }

        let expect = Some(((halfway + 1) * constants::LINE_SIZE, constants::BLOCK_SIZE));

        let got = meta.find_next_available_hole(0);

        println!("test_find_next_hole_at_block_end got {:?} expected {:?}", got, expect);

        assert!(got == expect);
    }

    #[test]
    fn test_find_hole_all_conservatively_marked() {
        let mut meta = BlockMeta::new_boxed();

        for i in 0..constants::LINE_COUNT {
            if i % 2 == 0 {  // there is no stable step function for range
                meta.mark_line(i);
            }
        }

        let got = meta.find_next_available_hole(0);

        println!("test_find_hole_all_conservatively_marked got {:?} expected None", got);

        assert!(got == None);
    }

    #[test]
    fn test_find_entire_block() {
        let meta = BlockMeta::new_boxed();

        let expect = Some((0, constants::BLOCK_SIZE));
        let got = meta.find_next_available_hole(0);

        println!("test_find_entire_block got {:?} expected {:?}", got, expect);

        assert!(got == expect);
    }
}
