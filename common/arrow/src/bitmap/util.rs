// Copyright 2022 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// This file is copied from
// https://github.com/jorgecarleitao/arrow2/blob/main/src/bitmap/utils/mod.rs
// and modified by Databend. We only reserve some functions which is useful
// to us.

const BIT_MASK: [u8; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
const UNSET_BIT_MASK: [u8; 8] = [
    255 - 1,
    255 - 2,
    255 - 4,
    255 - 8,
    255 - 16,
    255 - 32,
    255 - 64,
    255 - 128,
];

/// Returns whether bit at position `i` in `byte` is set or not
#[inline]
pub fn is_set(byte: u8, i: usize) -> bool {
    (byte & BIT_MASK[i]) != 0
}

/// Sets bit at position `i` in `byte`
#[inline]
pub fn set(byte: u8, i: usize, value: bool) -> u8 {
    if value {
        byte | BIT_MASK[i]
    } else {
        byte & UNSET_BIT_MASK[i]
    }
}

/// Sets bit at position `i` in `data`
/// # Panics
/// panics if `i >= data.len() / 8`
#[inline]
pub fn set_bit(data: &mut [u8], i: usize, value: bool) {
    data[i / 8] = set(data[i / 8], i % 8, value);
}

/// Sets bit at position `i` in `data` without doing bound checks
/// # Safety
/// caller must ensure that `i < data.len() / 8`
#[inline]
pub unsafe fn set_bit_unchecked(data: &mut [u8], i: usize, value: bool) {
    let byte = data.get_unchecked_mut(i / 8);
    *byte = set(*byte, i % 8, value);
}

/// Returns whether bit at position `i` in `data` is set or not
#[inline]
pub fn get_bit(data: &[u8], i: usize) -> bool {
    is_set(data[i / 8], i % 8)
}

/// Returns the number of zero bits in the slice offsetted by `offset` and a length of `length`.
/// # Panics
/// This function panics iff `(offset + len).saturating_add(7) / 8 >= slice.len()`
/// because it corresponds to the situation where `len` is beyond bounds.
pub fn count_zeros(slice: &[u8], offset: usize, len: usize) -> usize {
    if len == 0 {
        return 0;
    };

    let mut slice = &slice[offset / 8..(offset + len).saturating_add(7) / 8];
    let offset = offset % 8;

    if (offset + len) / 8 == 0 {
        // all within a single byte
        let byte = (slice[0] >> offset) << (8 - len);
        return len - byte.count_ones() as usize;
    }

    // slice: [a1,a2,a3,a4], [a5,a6,a7,a8]
    // offset: 3
    // len: 4
    // [__,__,__,a4], [a5,a6,a7,__]
    let mut set_count = 0;
    if offset != 0 {
        // count all ignoring the first `offset` bits
        // i.e. [__,__,__,a4]
        set_count += (slice[0] >> offset).count_ones() as usize;
        slice = &slice[1..];
    }
    if (offset + len) % 8 != 0 {
        let end_offset = (offset + len) % 8; // i.e. 3 + 4 = 7
        let last_index = slice.len() - 1;
        // count all ignoring the last `offset` bits
        // i.e. [a5,a6,a7,__]
        set_count += (slice[last_index] << (8 - end_offset)).count_ones() as usize;
        slice = &slice[..last_index];
    }

    // finally, count any and all bytes in the middle in groups of 8
    let mut chunks = slice.chunks_exact(8);
    set_count += chunks
        .by_ref()
        .map(|chunk| {
            let a = u64::from_ne_bytes(chunk.try_into().unwrap());
            a.count_ones() as usize
        })
        .sum::<usize>();

    // and any bytes that do not fit in the group
    set_count += chunks
        .remainder()
        .iter()
        .map(|byte| byte.count_ones() as usize)
        .sum::<usize>();

    len - set_count
}
