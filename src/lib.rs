use std::{mem::transmute, ops::RangeInclusive};

#[must_use]
#[inline]
const fn after_mask(bit_index: u32) -> u64 {
    u64::MAX << bit_index
}

#[must_use]
#[inline]
const fn bottom_mask(last_index: u32) -> u64 {
    u64::MAX >> (63 - last_index)
}

#[must_use]
#[inline]
const fn copy_range(range: &RangeInclusive<u8>) -> RangeInclusive<u8> {
    *range.start()..=*range.end()
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ByteSetMasks(u64, u64, u64, u64);

#[repr(C)]
#[derive(Clone, Copy)]
union ByteSetUnion {
    ints: [u64; 4],
    masks: ByteSetMasks,
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ByteSet(ByteSetUnion);

impl Default for ByteSet {
    #[inline]
    fn default() -> Self {
        Self::EMPTY
    }
}

impl std::fmt::Debug for ByteSet {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // SAFETY: Each union field has same layout and are both Copy.
        unsafe { std::fmt::Debug::fmt(&self.0.ints, f) }
    }
}

impl PartialEq<ByteSet> for ByteSet {
    #[inline]
    fn eq(&self, other: &ByteSet) -> bool {
        self.eq(other)
    }

    #[inline]
    fn ne(&self, other: &ByteSet) -> bool {
        self.ne(other)
    }
}

impl Eq for ByteSet {}

impl std::hash::Hash for ByteSet {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // SAFETY: Each union field has same layout and are both Copy.
        unsafe { self.0.masks.hash(state) }
    }
}

impl ByteSet {
    /// A [ByteSet] with no bytes added.
    pub const EMPTY: Self = Self(ByteSetUnion { ints: [0; 4] });
    pub const ASCII_LOWERCASE: Self = Self::from_range(b'a'..=b'z');
    pub const ASCII_UPPERCASE: Self = Self::from_range(b'A'..=b'Z');
    pub const ASCII_LETTERS: Self = Self::union(&[Self::ASCII_LOWERCASE, Self::ASCII_UPPERCASE]);
    pub const ASCII_DIGITS: Self = Self::from_range(b'0'..=b'9');
    pub const HEX_DIGITS: Self = Self::new()
        .with_range(b'a'..=b'f')
        .with_range(b'A'..=b'F')
        .with_range(b'0'..=b'9');
    pub const OCTAL_DIGITS: Self = Self::from_range(b'0'..=b'7');
    // 0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ!"#$%&'()*+,-./:;<=>?@[\]^_`{|}~
    pub const ASCII_PRINTABLE: Self = Self::new()
        .with_range(b'\t'..=b'\r')
        .with_range(b' '..=b'~');
    pub const ASCII_NON_PRINTABLE: Self = Self::ASCII_PRINTABLE.inverted();
    pub const ASCII_SYMBOLS: Self = Self::new()
        .with_range(b'!'..=b'/')
        .with_range(b':'..=b'@')
        .with_range(b'['..=b'`')
        .with_range(b'{'..=b'~');

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
    pub const fn union(sets: &[Self]) -> Self {
        let mut builder = Self::new();
        let mut index = 0;
        while index < sets.len() {
            builder.or_assign(sets[index]);
            index += 1;
        }
        builder
    }

    #[track_caller]
    #[must_use]
    #[inline(always)]
    pub const fn get_mask(&self, index: usize) -> u64 {
        unsafe { self.0.ints[index] }
    }

    #[must_use]
    #[inline(always)]
    pub const fn get_mask0(&self) -> u64 {
        unsafe { self.0.masks.0 }
    }

    #[must_use]
    #[inline(always)]
    pub const fn get_mask1(&self) -> u64 {
        unsafe { self.0.masks.1 }
    }

    #[must_use]
    #[inline(always)]
    pub const fn get_mask2(&self) -> u64 {
        unsafe { self.0.masks.2 }
    }

    #[must_use]
    #[inline(always)]
    pub const fn get_mask3(&self) -> u64 {
        unsafe { self.0.masks.3 }
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
    pub const fn from_range(range: RangeInclusive<u8>) -> Self {
        Self::new().with_range(range)
    }

    #[inline]
    pub const fn add_set(&mut self, set: Self) {
        self.or_assign(set);
    }

    pub const fn remove_set(&mut self, set: Self) {
        unsafe {
            self.0.masks.0 &= !set.0.masks.0;
            self.0.masks.1 &= !set.0.masks.1;
            self.0.masks.2 &= !set.0.masks.2;
            self.0.masks.3 &= !set.0.masks.3;
        }
    }

    pub const fn add_sets(&mut self, sets: &[Self]) {
        let mut index = 0;
        while index < sets.len() {
            self.add_set(sets[index]);
            index += 1;
        }
    }

    pub const fn remove_sets(&mut self, sets: &[Self]) {
        let mut index = 0;
        while index < sets.len() {
            self.remove_set(sets[index]);
            index += 1;
        }
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

    #[track_caller]
    #[inline]
    const fn internal_set_range<const VALUE: bool>(&mut self, first: u8, last: u8) {
        debug_assert!(last >= first);

        let mut mask_index = first as u32 / 64;
        let first_bit_index = first as u32 % 64;
        let first_mask = after_mask(first_bit_index);

        let last_mask_index = last as u32 / 64;
        let last_bit_index = last as u32 % 64;
        let last_mask = bottom_mask(last_bit_index);

        if mask_index == last_mask_index {
            let comb_mask = first_mask & last_mask;
            unsafe {
                if const{ VALUE } {
                    self.0.ints[mask_index as usize] |= comb_mask;
                } else {
                    self.0.ints[mask_index as usize] &= !comb_mask;
                }
            }
            return;
        }

        unsafe {
            if const { VALUE } {
                self.0.ints[mask_index as usize] |= first_mask;
                self.0.ints[last_mask_index as usize] |= last_mask;
            } else {
                self.0.ints[mask_index as usize] &= !first_mask;
                self.0.ints[last_mask_index as usize] &= !last_mask;
            }
        }

        mask_index += 1;
        while mask_index < last_mask_index {
            unsafe {
                self.0.ints[mask_index as usize] = const { if VALUE  {
                    u64::MAX
                } else {
                    0
                }};
            }
            mask_index += 1;
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
    pub const fn add_range(&mut self, range: RangeInclusive<u8>) {
        self.internal_set_range::<true>(*range.start(), *range.end());
    }

    #[track_caller]
    pub const fn remove_range(&mut self, range: RangeInclusive<u8>) {
        self.internal_set_range::<false>(*range.start(), *range.end());
    }

    #[track_caller]
    pub const fn add_ranges(&mut self, ranges: &[RangeInclusive<u8>]) {
        let mut index = 0;
        while index < ranges.len() {
            self.add_range(copy_range(&ranges[index]));
            index += 1;
        }
    }

    #[track_caller]
    pub const fn remove_ranges(&mut self, ranges: &[RangeInclusive<u8>]) {
        let mut index = 0;
        while index < ranges.len() {
            self.remove_range(copy_range(&ranges[index]));
            index += 1;
        }
    }

    #[track_caller]
    pub const fn set_range(&mut self, range: RangeInclusive<u8>, value: bool) {
        if value {
            self.add_range(range);
        } else {
            self.remove_range(range);
        }
    }

    #[must_use]
    #[inline]
    pub const fn with_set(mut self, set: Self) -> Self {
        self.add_set(set);
        self
    }

    #[must_use]
    #[inline]
    pub const fn without_set(mut self, set: Self) -> Self {
        self.remove_set(set);
        self
    }

    #[must_use]
    #[inline]
    pub const fn with_sets(mut self, sets: &[Self]) -> Self {
        self.add_sets(sets);
        self
    }

    #[must_use]
    #[inline]
    pub const fn without_sets(mut self, sets: &[Self]) -> Self {
        self.remove_sets(sets);
        self
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

    #[track_caller]
    #[must_use]
    #[inline]
    pub const fn with_range(mut self, range: RangeInclusive<u8>) -> Self {
        self.add_range(range);
        self
    }

    #[track_caller]
    #[must_use]
    #[inline]
    pub const fn without_range(mut self, range: RangeInclusive<u8>) -> Self {
        self.remove_range(range);
        self
    }

    #[track_caller]
    #[must_use]
    #[inline]
    pub const fn with_ranges(mut self, ranges: &[RangeInclusive<u8>]) -> Self {
        self.add_ranges(ranges);
        self
    }

    #[track_caller]
    #[must_use]
    #[inline]
    pub const fn without_ranges(mut self, ranges: &[RangeInclusive<u8>]) -> Self {
        self.remove_ranges(ranges);
        self
    }

    #[must_use]
    #[inline]
    pub const fn has(&self, bit: u8) -> bool {
        let mask_index = bit as usize / 64;
        let bit_index = bit as u32 % 64;
        self.get_mask(mask_index) & (1 << bit_index) != 0
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
                if missing { return true; }
                found = true;
            } else {
                if found { return true; }
                missing = true;
            }
            index += 1;
        }
        false
    }

    #[must_use]
    #[inline]
    pub const fn has_char(&self, c: char) -> bool {
        (c as u32) < 256 && self.has(c as u8)
    }

    #[must_use]
    pub const fn has_any_char(&self, chars: &[char]) -> bool {
        let mut index = 0;
        while index < chars.len() {
            if self.has_char(chars[index]) {
                return true;
            }
            index += 1;
        }
        false
    }

    #[must_use]
    pub const fn has_all_chars(&self, chars: &[char]) -> bool {
        let mut index = 0;
        while index < chars.len() {
            if !self.has_char(chars[index]) {
                return false;
            }
            index += 1;
        }
        true
    }

    #[must_use]
    pub const fn has_some_chars(&self, chars: &[char]) -> bool {
        let mut missing = false;
        let mut found = false;
        let mut index = 0;
        while index < chars.len() {
            if self.has_char(chars[index]) {
                if missing { return true; }
                found = true;
            } else {
                if found { return true; }
                missing = true;
            }
            index += 1;
        }
        false
    }

    pub const fn eq(&self, other: &Self) -> bool {
        self.get_mask0() == other.get_mask0() &&
        self.get_mask1() == other.get_mask1() &&
        self.get_mask2() == other.get_mask2() &&
        self.get_mask3() == other.get_mask3()
    }

    pub const fn ne(&self, other: &Self) -> bool {
        self.get_mask0() != other.get_mask0() ||
        self.get_mask1() != other.get_mask1() ||
        self.get_mask2() != other.get_mask2() ||
        self.get_mask3() != other.get_mask3()
    }

    #[must_use]
    pub const fn is_disjoint(&self, other: &Self) -> bool {
        #[inline]
        const fn is_disjoint(lhs: u64, rhs: u64) -> bool {
            0 == lhs & rhs
        }
        is_disjoint(self.get_mask0(), other.get_mask0()) &&
        is_disjoint(self.get_mask1(), other.get_mask1()) &&
        is_disjoint(self.get_mask2(), other.get_mask2()) &&
        is_disjoint(self.get_mask3(), other.get_mask3())
    }

    #[must_use]
    pub const fn intersects(&self, other: &Self) -> bool {
        #[must_use]
        #[inline]
        const fn intersects(lhs: u64, rhs: u64) -> bool {
            0 != lhs & rhs
        }
        intersects(self.get_mask0(), other.get_mask0()) ||
        intersects(self.get_mask1(), other.get_mask1()) ||
        intersects(self.get_mask2(), other.get_mask2()) ||
        intersects(self.get_mask3(), other.get_mask3())
    }

    #[must_use]
    pub const fn is_superset(&self, other: &Self) -> bool {
        #[must_use]
        #[inline(always)]
        const fn is_superset(lhs: u64, rhs: u64) -> bool {
            lhs & rhs == rhs
        }
        is_superset(self.get_mask0(), other.get_mask0()) &&
        is_superset(self.get_mask1(), other.get_mask1()) &&
        is_superset(self.get_mask2(), other.get_mask2()) &&
        is_superset(self.get_mask3(), other.get_mask3())
    }

    #[must_use]
    pub const fn is_subset(&self, other: &Self) -> bool {
        #[must_use]
        #[inline(always)]
        const fn is_subset(lhs: u64, rhs: u64) -> bool {
            lhs & rhs == lhs
        }
        is_subset(self.get_mask0(), other.get_mask0()) &&
        is_subset(self.get_mask1(), other.get_mask1()) &&
        is_subset(self.get_mask2(), other.get_mask2()) &&
        is_subset(self.get_mask3(), other.get_mask3())
    }

    #[track_caller]
    #[must_use]
    #[inline]
    pub const fn range_subset(self, range: RangeInclusive<u8>) -> Self {
        self.and(ByteSet::from_range(range))
    }

    #[must_use]
    #[inline]
    pub const fn iter(&self) -> ByteSetIter<'_> {
        ByteSetIter { set: self, index: 0 }
    }

    #[must_use]
    #[inline]
    pub const fn to_ne_bytes(&self) -> [u8; 32] {
        unsafe {
            transmute([
                self.get_mask0().to_ne_bytes(),
                self.get_mask1().to_ne_bytes(),
                self.get_mask2().to_ne_bytes(),
                self.get_mask3().to_ne_bytes(),
            ])
        }
    }

    #[must_use]
    #[inline]
    pub const fn to_le_bytes(&self) -> [u8; 32] {
        unsafe {
            transmute([
                self.get_mask0().to_le_bytes(),
                self.get_mask1().to_le_bytes(),
                self.get_mask2().to_le_bytes(),
                self.get_mask3().to_le_bytes(),
            ])
        }
    }

    #[must_use]
    #[inline]
    pub const fn to_be_bytes(&self) -> [u8; 32] {
        unsafe {
            transmute([
                self.get_mask0().to_be_bytes(),
                self.get_mask1().to_be_bytes(),
                self.get_mask2().to_be_bytes(),
                self.get_mask3().to_be_bytes(),
            ])
        }
    }

    #[must_use]
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
    pub const fn to_array(self) -> [u64; 4] {
        unsafe { self.0.ints }
    }

    pub const fn or_assign(&mut self, other: Self) {
        unsafe {
            self.0.masks.0 |= other.get_mask0();
            self.0.masks.1 |= other.get_mask1();
            self.0.masks.2 |= other.get_mask2();
            self.0.masks.3 |= other.get_mask3();
        }
    }

    pub const fn or(mut self, other: Self) -> Self {
        self.or_assign(other);
        self
    }

    pub const fn and_assign(&mut self, other: Self) {
        unsafe {
            self.0.masks.0 &= other.get_mask0();
            self.0.masks.1 &= other.get_mask1();
            self.0.masks.2 &= other.get_mask2();
            self.0.masks.3 &= other.get_mask3();
        }
    }

    pub const fn and(mut self, other: Self) -> Self {
        self.and_assign(other);
        self
    }

    pub const fn xor_assign(&mut self, other: Self) {
        unsafe {
            self.0.masks.0 ^= other.get_mask0();
            self.0.masks.1 ^= other.get_mask1();
            self.0.masks.2 ^= other.get_mask2();
            self.0.masks.3 ^= other.get_mask3();
        }
    }

    pub const fn xor(mut self, other: Self) -> Self {
        self.xor_assign(other);
        self
    }

    pub const fn invert(&mut self) {
        unsafe {
            self.0.masks.0 = !self.0.masks.0;
            self.0.masks.1 = !self.0.masks.1;
            self.0.masks.2 = !self.0.masks.2;
            self.0.masks.3 = !self.0.masks.3;
        }
    }

    #[must_use]
    #[inline]
    pub const fn inverted(mut self) -> Self {
        self.invert();
        self
    }
}

impl std::ops::Not for ByteSet {
    type Output = Self;
    #[inline]
    fn not(self) -> Self::Output {
        self.inverted()
    }
}

impl std::ops::BitOr<Self> for ByteSet {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        self.or(rhs)
    }
}

impl std::ops::BitOrAssign<Self> for ByteSet {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.or_assign(rhs);
    }
}

impl std::ops::BitAnd<Self> for ByteSet {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        self.and(rhs)
    }
}

impl std::ops::BitAndAssign<Self> for ByteSet {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.and_assign(rhs);
    }
}

impl std::ops::BitXor<Self> for ByteSet {
    type Output = Self;
    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        self.xor(rhs)
    }
}

impl std::ops::BitXorAssign<Self> for ByteSet {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.xor_assign(rhs);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ByteSetIter<'a> {
    set: &'a ByteSet,
    index: u32,
}

impl<'a> ByteSetIter<'a> {

    pub const fn remaining(&self) -> usize {
        let mut mask_index = (self.index / 64) as usize;
        let bit_index = self.index % 64;
        let after_mask = after_mask(bit_index);
        let mut count = (self.set.get_mask(mask_index) & after_mask).count_ones();
        mask_index += 1;
        while mask_index < 4 {
            count += self.set.get_mask(mask_index).count_ones();
            mask_index += 1;
        }
        count as usize
    }

    pub const fn next(&mut self) -> Option<u8> {
        let mut index = self.index;
        let mut mask_index = index / 64;
        loop {
            if mask_index >= 4 {
                // commit index.
                self.index = index;
                return None;
            }
            let bit_index = index % 64;
            let mask = after_mask(bit_index);
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

impl<'a> Iterator for ByteSetIter<'a> {
    type Item = u8;
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining();
        (remaining, Some(remaining))
    }

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        ByteSetIter::next(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn byteset_iter() {
        let set = ByteSet::from_range(48..=80).without_range(60..=68);
        let subset = set.and(ByteSet::from_range(56..=72));
        let it = ByteSetIter {
            set: &subset,
            index: 0,
        };
        assert_eq!(it.remaining(), 8);
        let bytes = it.collect::<Vec<u8>>();
        assert_eq!(bytes.len(), bytes.capacity());
        assert_eq!(bytes.len(), 8);
        // while let Some(byte) = it.next() {
        //     println!("{byte}");
        // }
    }
}