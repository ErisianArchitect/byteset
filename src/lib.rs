use std::{mem::transmute};

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ByteSetMasks(u64, u64, u64, u64);

#[derive(Clone, Copy)]
union ByteSetUnion {
    ints: [u64; 4],
    masks: ByteSetMasks,
}

#[derive(Clone, Copy)]
pub struct ByteSet(ByteSetUnion);

impl Default for ByteSet {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl std::fmt::Debug for ByteSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // SAFETY: Each union field has same layout and are both Copy.
        unsafe { std::fmt::Debug::fmt(&self.0.ints, f) }
    }
}

impl PartialEq<ByteSet> for ByteSet {
    fn eq(&self, other: &ByteSet) -> bool {
        // SAFETY: Each union field has same layout and are both Copy.
        unsafe { self.0.masks == other.0.masks }
    }

    fn ne(&self, other: &ByteSet) -> bool {
        // SAFETY: Each union field has same layout and are both Copy.
        unsafe { self.0.masks != other.0.masks }
    }
}

impl Eq for ByteSet {}

impl std::hash::Hash for ByteSet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // SAFETY: Each union field has same layout and are both Copy.
        unsafe { self.0.masks.hash(state) }
    }
}

impl ByteSet {
    /// A [ByteSet] with no bytes added.
    pub const EMPTY: Self = Self(ByteSetUnion { ints: [0; 4] });

    #[must_use]
    #[inline]
    pub const fn is_empty(&self) -> bool {
        unsafe {
            self.0.masks.0 == u64::MIN &&
            self.0.masks.1 == u64::MIN &&
            self.0.masks.2 == u64::MIN &&
            self.0.masks.3 == u64::MIN
        }
    }

    #[must_use]
    #[inline]
    pub const fn is_full(&self) -> bool {
        unsafe {
            self.0.masks.0 == u64::MAX &&
            self.0.masks.1 == u64::MAX &&
            self.0.masks.2 == u64::MAX &&
            self.0.masks.3 == u64::MAX
        }
    }

    #[must_use]
    #[inline]
    pub const fn len(&self) -> usize {
        (
            unsafe {
                self.0.masks.0.count_ones() +
                self.0.masks.1.count_ones() +
                self.0.masks.2.count_ones() +
                self.0.masks.3.count_ones()
            }
        ) as usize
    }

    #[must_use]
    #[inline(always)]
    pub const fn new() -> Self {
        Self::EMPTY
    }

    #[must_use]
    #[inline(always)]
    pub const fn from_array(array: [u64; 4]) -> Self {
        Self(ByteSetUnion { ints: array })
    }

    #[must_use]
    #[inline(always)]
    pub const fn from_bytes(bytes: &[u8]) -> Self {
        Self::new().with_bytes(bytes)
    }

    #[track_caller]
    #[must_use]
    #[inline(always)]
    pub const fn from_range(first: u8, last: u8) -> Self {
        Self::new().with_range(first, last)
    }

    #[inline]
    const fn internal_set_bit_to<const VALUE: bool>(&mut self, index: u8) {
        let mask_index = index as usize / 64;
        let bit_index = index as u32 % 64;
        unsafe {
            if const { VALUE } {
                self.0.ints[mask_index] |= 1 << bit_index;
            } else {
                self.0.ints[mask_index] &= !(1 << bit_index);
            }
        }
    }

    #[inline]
    const fn internal_set_bytes<const VALUE: bool>(&mut self, bytes: &[u8]) {
        let mut index = 0;
        while index < bytes.len() {
            let c = bytes[index];
            self.internal_set_bit_to::<VALUE>(c);
            index += 1;
        }
    }

    pub const fn add_byte(&mut self, byte: u8) {
        self.internal_set_bit_to::<true>(byte);
    }
    
    pub const fn remove_byte(&mut self, byte: u8) {
        self.internal_set_bit_to::<false>(byte)
    }

    pub const fn add_bytes(&mut self, bytes: &[u8]) {
        self.internal_set_bytes::<true>(bytes);
    }

    pub const fn remove_bytes(&mut self, bytes: &[u8]) {
        self.internal_set_bytes::<false>(bytes);
    }

    pub const fn set_byte(&mut self, byte: u8, value: bool) {
        if value {
            self.add_byte(byte);
        } else {
            self.remove_byte(byte);
        }
    }

    pub const fn set_bytes(&mut self, bytes: &[u8], value: bool) {
        if value {
            self.add_bytes(bytes);
        } else {
            self.remove_bytes(bytes);
        }
    }

    #[track_caller]
    #[inline]
    const fn internal_set_range<const VALUE: bool>(&mut self, first: u8, last: u8) {
        debug_assert!(last >= first);
        let mut index = first;
        loop {
            self.internal_set_bit_to::<VALUE>(index);
            if index == last {
                break;
            }
            index += 1;
        }
    }

    #[track_caller]
    pub const fn add_range(&mut self, first: u8, last: u8) {
        self.internal_set_range::<true>(first, last);
    }

    #[track_caller]
    pub const fn remove_range(&mut self, first: u8, last: u8) {
        self.internal_set_range::<false>(first, last);
    }

    #[track_caller]
    pub const fn set_range(&mut self, first: u8, last: u8, value: bool) {
        if value {
            self.add_range(first, last);
        } else {
            self.remove_range(first, last);
        }
    }

    #[must_use]
    #[inline]
    pub const fn with_byte(mut self, byte: u8) -> Self {
        self.add_byte(byte);
        self
    }

    #[must_use]
    #[inline]
    pub const fn without_byte(mut self, byte: u8) -> Self {
        self.remove_byte(byte);
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_bytes(mut self, bytes: &[u8]) -> Self {
        self.add_bytes(bytes);
        self
    }

    #[must_use]
    #[inline]
    pub const fn without_bytes(mut self, bytes: &[u8]) -> Self {
        self.remove_bytes(bytes);
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_range(mut self, first: u8, last: u8) -> Self {
        self.add_range(first, last);
        self
    }

    #[must_use]
    #[inline]
    pub const fn without_range(mut self, first: u8, last: u8) -> Self {
        self.remove_range(first, last);
        self
    }

    #[must_use]
    #[inline]
    pub const fn has(&self, bit: u8) -> bool {
        let mask_index = bit as usize / 64;
        let bit_index = bit as u32 % 64;
        unsafe { (self.0.ints[mask_index] & (1 << bit_index)) != 0 }
    }

    #[must_use]
    pub const fn has_any(&self, bytes: &[u8]) -> bool {
        let mut index = 0;
        while index < bytes.len() {
            if self.has(bytes[index]) {
                return true;
            }
            index += 1;
        }
        false
    }

    #[must_use]
    pub const fn has_all(&self, bytes: &[u8]) -> bool {
        let mut index = 0;
        while index < bytes.len() {
            if !self.has(bytes[index]) {
                return false;
            }
            index += 1;
        }
        true
    }

    /// Tests if the set contains some but not all `bytes`.
    #[must_use]
    pub const fn has_some(&self, bytes: &[u8]) -> bool {
        let mut missing = false;
        let mut found = false;
        let mut index = 0;
        while index < bytes.len() {
            if self.has(bytes[index]) {
                if missing {
                    return true;
                }
                found = true;
            } else {
                if found {
                    return true;
                }
                missing = true;
            }
            index += 1;
        }
        false
    }

    pub const fn eq(&self, other: &Self) -> bool {
        unsafe {
            self.0.masks.0 == other.0.masks.0 &&
            self.0.masks.1 == other.0.masks.1 &&
            self.0.masks.2 == other.0.masks.2 &&
            self.0.masks.3 == other.0.masks.3
        }
    }

    pub const fn ne(&self, other: &Self) -> bool {
        unsafe {
            self.0.masks.0 != other.0.masks.0 ||
            self.0.masks.1 != other.0.masks.1 ||
            self.0.masks.2 != other.0.masks.2 ||
            self.0.masks.3 != other.0.masks.3
        }
    }

    #[must_use]
    pub const fn is_disjoint(&self, other: &Self) -> bool {
        #[inline]
        const fn is_disjoint(lhs: u64, rhs: u64) -> bool {
            0 == lhs & rhs
        }
        unsafe {
            is_disjoint(self.0.masks.0, other.0.masks.0) &&
            is_disjoint(self.0.masks.1, other.0.masks.1) &&
            is_disjoint(self.0.masks.2, other.0.masks.2) &&
            is_disjoint(self.0.masks.3, other.0.masks.3)
        }
    }

    #[must_use]
    pub const fn intersects(&self, other: &Self) -> bool {
        #[inline]
        const fn intersects(lhs: u64, rhs: u64) -> bool {
            0 != lhs & rhs
        }
        unsafe {
            intersects(self.0.masks.0, other.0.masks.0) ||
            intersects(self.0.masks.1, other.0.masks.1) ||
            intersects(self.0.masks.2, other.0.masks.2) ||
            intersects(self.0.masks.3, other.0.masks.3)
        }
    }

    #[must_use]
    pub const fn is_superset(&self, other: &Self) -> bool {
        #[inline(always)]
        const fn is_superset(lhs: u64, rhs: u64) -> bool {
            lhs & rhs == rhs
        }
        unsafe {
            is_superset(self.0.masks.0, other.0.masks.0) &&
            is_superset(self.0.masks.1, other.0.masks.1) &&
            is_superset(self.0.masks.2, other.0.masks.2) &&
            is_superset(self.0.masks.3, other.0.masks.3)
        }
    }

    #[must_use]
    pub const fn is_subset(&self, other: &Self) -> bool {
        #[inline(always)]
        const fn is_subset(lhs: u64, rhs: u64) -> bool {
            lhs & rhs == lhs
        }
        unsafe {
            is_subset(self.0.masks.0, other.0.masks.0) &&
            is_subset(self.0.masks.1, other.0.masks.1) &&
            is_subset(self.0.masks.2, other.0.masks.2) &&
            is_subset(self.0.masks.3, other.0.masks.3)
        }
    }

    #[must_use]
    pub const fn iter(&self) -> ByteSetIter<'_> {
        ByteSetIter { set: self, index: 0 }
    }

    #[must_use]
    pub const fn to_ne_bytes(&self) -> [u8; 32] {
        unsafe {
            transmute([
                self.0.masks.0.to_ne_bytes(),
                self.0.masks.1.to_ne_bytes(),
                self.0.masks.2.to_ne_bytes(),
                self.0.masks.3.to_ne_bytes(),
            ])
        }
    }

    #[must_use]
    pub const fn to_le_bytes(&self) -> [u8; 32] {
        unsafe {
            transmute([
                self.0.masks.0.to_le_bytes(),
                self.0.masks.1.to_le_bytes(),
                self.0.masks.2.to_le_bytes(),
                self.0.masks.3.to_le_bytes(),
            ])
        }
    }

    #[must_use]
    pub const fn to_be_bytes(&self) -> [u8; 32] {
        unsafe {
            transmute([
                self.0.masks.0.to_be_bytes(),
                self.0.masks.1.to_be_bytes(),
                self.0.masks.2.to_be_bytes(),
                self.0.masks.3.to_be_bytes(),
            ])
        }
    }

    #[must_use]
    pub const fn from_ne_bytes(bytes: [u8; 32]) -> Self {
        let [c0, c1, c2, c3]: [[u8; 8]; 4] = unsafe { transmute(bytes) };
        Self(ByteSetUnion { masks: ByteSetMasks(
            u64::from_ne_bytes(c0),
            u64::from_ne_bytes(c1),
            u64::from_ne_bytes(c2),
            u64::from_ne_bytes(c3),
        )})
    }

    #[must_use]
    pub const fn from_le_bytes(bytes: [u8; 32]) -> Self {
        let [c0, c1, c2, c3]: [[u8; 8]; 4] = unsafe { transmute(bytes) };
        Self(ByteSetUnion { masks: ByteSetMasks(
            u64::from_le_bytes(c0),
            u64::from_le_bytes(c1),
            u64::from_le_bytes(c2),
            u64::from_le_bytes(c3),
        )})
    }

    #[must_use]
    pub const fn from_be_bytes(bytes: [u8; 32]) -> Self {
        let [c0, c1, c2, c3]: [[u8; 8]; 4] = unsafe { transmute(bytes) };
        Self(ByteSetUnion { masks: ByteSetMasks(
            u64::from_be_bytes(c0),
            u64::from_be_bytes(c1),
            u64::from_be_bytes(c2),
            u64::from_be_bytes(c3),
        )})
    }

    #[must_use]
    pub const fn to_array(self) -> [u64; 4] {
        unsafe { self.0.ints }
    }

    pub const fn or_assign(&mut self, other: &Self) {
        unsafe {
            self.0.masks.0 |= other.0.masks.0;
            self.0.masks.1 |= other.0.masks.1;
            self.0.masks.2 |= other.0.masks.2;
            self.0.masks.3 |= other.0.masks.3;
        }
    }

    pub const fn or(mut self, other: Self) -> Self {
        self.or_assign(&other);
        self
    }

    pub const fn and_assign(&mut self, other: &Self) {
        unsafe {
            self.0.masks.0 &= other.0.masks.0;
            self.0.masks.1 &= other.0.masks.1;
            self.0.masks.2 &= other.0.masks.2;
            self.0.masks.3 &= other.0.masks.3;
        }
    }

    pub const fn and(mut self, other: Self) -> Self {
        self.and_assign(&other);
        self
    }

    pub const fn xor_assign(&mut self, other: &Self) {
        unsafe {
            self.0.masks.0 ^= other.0.masks.0;
            self.0.masks.1 ^= other.0.masks.1;
            self.0.masks.2 ^= other.0.masks.2;
            self.0.masks.3 ^= other.0.masks.3;
        }
    }

    pub const fn xor(mut self, other: Self) -> Self {
        self.xor_assign(&other);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteSetIter<'a> {
    set: &'a ByteSet,
    index: u32,
}

impl<'a> Iterator for ByteSetIter<'a> {
    type Item = u8;
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.set.len(), Some(self.set.len()))
    }

    fn next(&mut self) -> Option<Self::Item> {
        let mut index = self.index;
        let mut mask_index = index / 64;
        loop {
            if mask_index >= 4 {
                // commit index.
                self.index = index;
                return None;
            }
            let bit_index = index % 64;
            let mask = u64::MAX << bit_index;
            let masked = unsafe { self.set.0.ints[mask_index as usize] } & mask;
            let next_1 = masked.trailing_zeros();
            index = mask_index * 64 + next_1;
            if next_1 != 64 {
                self.index = index + 1;
                return Some(index as u8);
            }
            mask_index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn byteset_iter() {
        let set = ByteSet::from_range(0, 10);
        let it = ByteSetIter {
            set: &set,
            index: 0,
        };
        for byte in it {
            println!("{byte}");
        }
    }
}