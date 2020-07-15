#[derive(Debug, PartialEq)]
pub struct Bitfield(Vec<u8>);

//
// we count from the left here
//

impl Bitfield {
    pub fn has_piece(&self, index: usize) -> bool {
        let byte_index = index / 8;
        let byte_offset = index % 8;
        if self.0.len() <= byte_index {
            return false;
        }
        return self.0[byte_index] >> (7 - byte_offset) & 1 != 0;
    }

    pub fn set_piece(&mut self, index: usize) {
        let byte_index = index / 8;
        let byte_offset = index % 8;
        if self.0.len() <= byte_index {
            return;
        }
        //self.0[byte_index] |= 1 << (7 - byte_offset);
        self.0[byte_index] |= 1 << (7 - byte_offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_piece() {
        let bitfield = Bitfield(vec![0b01010100, 0b01010100]);
        let outputs = vec![
            false, true, false, true, false, true, false, false, false, true, false, true, false,
            true, false, false, false, false, false, false,
        ];
        for (i, output) in outputs.iter().enumerate() {
            assert_eq!(output, &bitfield.has_piece(i));
        }
    }

    #[test]
    fn test_set_piece() {
        // 5th bit set
        let mut input = Bitfield(vec![0b01010100, 0b01010100]);
        let index = 4;
        let output = Bitfield(vec![0b01011100, 0b01010100]);
        input.set_piece(index);
        assert_eq!(input, output);

        // no-op
        let mut input = Bitfield(vec![0b01010100, 0b01010100]);
        let index = 9;
        let output = Bitfield(vec![0b01010100, 0b01010100]);
        input.set_piece(index);
        assert_eq!(input, output);

        // 16th bit changes
        let mut input = Bitfield(vec![0b01010100, 0b01010100]);
        let index = 15;
        let output = Bitfield(vec![0b01010100, 0b01010101]);
        input.set_piece(index);
        assert_eq!(input, output);

        // 20th bit out-of-range
        let mut input = Bitfield(vec![0b01010100, 0b01010100]);
        let index = 19;
        let output = Bitfield(vec![0b01010100, 0b01010100]);
        input.set_piece(index);
        assert_eq!(input, output);
    }
}
