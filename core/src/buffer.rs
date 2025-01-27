// This file is part of Gear.

// Copyright (C) 2021-2025 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Vector with limited len realization.

use core::{
    convert::TryFrom,
    fmt::{self, Debug, Display, Formatter},
    marker::PhantomData,
};

use alloc::{vec, vec::Vec};
use parity_scale_codec::MaxEncodedLen;
use scale_info::{
    scale::{Decode, Encode},
    TypeInfo,
};

/// A dedicated trait to format data in [`LimitedVec`].
///
/// If data can be represented as bytes then it is formatted in limited format,
/// otherwise only fraction of elements in a vec is formatted.
pub trait LimitedDebug: Sized + Debug {
    /// Format the data in limited format.
    ///
    /// Limited format is a format that shows only a fraction of elements in a vec.
    /// The fraction is determined by the precision of the formatter.
    fn fmt<E, const N: usize>(vec: &LimitedVec<Self, E, N>, f: &mut Formatter<'_>) -> fmt::Result {
        let len = vec.0.len();
        let median = (len + 1) / 2;

        let mut e1 = median;
        let mut s2 = median;

        if let Some(precision) = f.precision() {
            if precision < median {
                e1 = precision;
                s2 = len - precision;
            } else if !f.sign_plus() && median > 8 {
                e1 = 8;
                s2 = len - 8;
            }
        }

        write!(f, "LimitedVec(")?;

        for (i, element) in vec.0[..e1].iter().enumerate() {
            <Self as Debug>::fmt(element, f)?;
            if i < e1 - 1 {
                write!(f, ",")?;
            }
        }

        let sep = e1.ne(&s2).then_some("..").unwrap_or_default();
        write!(f, "{sep}")?;

        for (i, element) in vec.0[s2..].iter().enumerate() {
            <Self as Debug>::fmt(element, f)?;
            if i < len - 1 {
                write!(f, ",")?;
            }
        }

        write!(f, ")")
    }
}

/// Same as [`LimitedDebug`] but for [`Display`] trait.
pub trait LimitedDisplay: Sized + Display {
    /// Format the data in limited format.
    ///
    /// Limited format is a format that shows only a fraction of elements in a vec.
    /// The fraction is determined by the precision of the formatter.
    fn fmt<E, const N: usize>(vec: &LimitedVec<Self, E, N>, f: &mut Formatter<'_>) -> fmt::Result {
        let len = vec.0.len();
        let median = (len + 1) / 2;

        let mut e1 = median;
        let mut s2 = median;

        if let Some(precision) = f.precision() {
            if precision < median {
                e1 = precision;
                s2 = len - precision;
            }
        } else if !f.sign_plus() && median > 8 {
            e1 = 8;
            s2 = len - 8;
        }

        write!(f, "LimitedVec(")?;

        for (i, element) in vec.0[..e1].iter().enumerate() {
            <Self as Display>::fmt(element, f)?;
            if i < e1 - 1 {
                write!(f, ",")?;
            }
        }

        let sep = e1.ne(&s2).then_some("..").unwrap_or_default();
        write!(f, "{sep}")?;

        for (i, element) in vec.0[s2..].iter().enumerate() {
            <Self as Display>::fmt(element, f)?;
            if i < len - 1 {
                write!(f, ",")?;
            }
        }

        write!(f, ")")
    }
}

/* override default formatting for bytes to print `LimitedVec(0x123..4643)` */
impl LimitedDebug for u8 {
    fn fmt<E, const N: usize>(vec: &LimitedVec<Self, E, N>, f: &mut Formatter<'_>) -> fmt::Result {
        let len = vec.0.len();
        let median = (len + 1) / 2;

        let mut e1 = median;
        let mut s2 = median;

        if let Some(precision) = f.precision() {
            if precision < median {
                e1 = precision;
                s2 = len - precision;
            }
        } else if !f.sign_plus() && median > 8 {
            e1 = 8;
            s2 = len - 8;
        }

        let p1 = hex::encode(&vec.0[..e1]);
        let p2 = hex::encode(&vec.0[s2..]);
        let sep = e1.ne(&s2).then_some("..").unwrap_or_default();

        if f.alternate() {
            write!(f, "LimitedVec(0x{p1}{sep}{p2})")
        } else {
            write!(f, "0x{p1}{sep}{p2}")
        }
    }
}

impl LimitedDisplay for u8 {
    fn fmt<E, const N: usize>(vec: &LimitedVec<Self, E, N>, f: &mut Formatter<'_>) -> fmt::Result {
        <u8 as LimitedDebug>::fmt(vec, f)
    }
}

impl<A, B> LimitedDebug for (A, B)
where
    A: Debug,
    B: Debug,
{
}

/// Limited len vector.
/// `T` is data type.
/// `E` is overflow error type.
/// `N` is max len which a vector can have.
#[derive(Clone, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Decode, Encode, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct LimitedVec<T, E, const N: usize>(Vec<T>, PhantomData<E>);

/// Formatter for [`LimitedVec`] will print to precision of 8 by default, to print the whole data, use `{:+}`.
impl<T: Clone + Default, E: Default, const N: usize> Display for LimitedVec<T, E, N>
where
    T: LimitedDisplay,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        <T as LimitedDisplay>::fmt(self, f)
    }
}

impl<T: Clone + Default, E: Default, const N: usize> Debug for LimitedVec<T, E, N>
where
    T: LimitedDebug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        <T as LimitedDebug>::fmt(self, f)
    }
}

impl<T, E: Default, const N: usize> TryFrom<Vec<T>> for LimitedVec<T, E, N> {
    type Error = E;
    fn try_from(x: Vec<T>) -> Result<Self, Self::Error> {
        (x.len() <= N).then_some(()).ok_or_else(E::default)?;
        Ok(Self(x, PhantomData))
    }
}

impl<T: Clone + Default, E: Default, const N: usize> LimitedVec<T, E, N> {
    /// Constructs a new, empty `LimitedVec<T>`.
    pub const fn new() -> Self {
        Self(Vec::new(), PhantomData)
    }

    /// Tries to create new limited vector of length `len`
    /// with default initialized elements.
    pub fn try_new_default(len: usize) -> Result<Self, E> {
        (len <= N).then_some(()).ok_or_else(E::default)?;
        Ok(Self(vec![T::default(); len], PhantomData))
    }

    /// Creates new limited vector with default initialized elements.
    pub fn new_default() -> Self {
        Self(vec![T::default(); N], PhantomData)
    }

    /// Creates limited vector filled with the specified `value`.
    pub fn filled_with(value: T) -> Self {
        Self(vec![value; N], PhantomData)
    }

    /// Extends the array to its limit and fills with the specified `value`.
    pub fn extend_with(&mut self, value: T) {
        self.0.resize(N, value);
    }

    /// Append `value` to the end of vector.
    pub fn try_push(&mut self, value: T) -> Result<(), E> {
        (self.0.len() != N).then_some(()).ok_or_else(E::default)?;
        self.0.push(value);
        Ok(())
    }

    /// Append `values` to the end of vector.
    pub fn try_extend_from_slice(&mut self, values: &[T]) -> Result<(), E> {
        self.0
            .len()
            .checked_add(values.len())
            .and_then(|len| (len <= N).then_some(()))
            .ok_or_else(E::default)?;

        self.0.extend_from_slice(values);

        Ok(())
    }

    /// Append `values` to the begin of vector.
    pub fn try_prepend(&mut self, values: Self) -> Result<(), E> {
        self.0
            .len()
            .checked_add(values.0.len())
            .and_then(|len| (len <= N).then_some(()))
            .ok_or_else(E::default)?;

        self.0.splice(0..0, values.0);

        Ok(())
    }

    /// Returns ref to the internal data.
    pub fn inner(&self) -> &[T] {
        &self.0
    }

    /// Returns mut ref to the internal data slice.
    pub fn inner_mut(&mut self) -> &mut [T] {
        &mut self.0
    }

    /// Destruct limited vector and returns inner vector.
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }

    /// Returns max len which this type of limited vector can have.
    pub const fn max_len() -> usize {
        N
    }
}

impl<T: MaxEncodedLen, E, const N: usize> MaxEncodedLen for LimitedVec<T, E, N> {
    fn max_encoded_len() -> usize {
        N * T::max_encoded_len()
    }
}

/// Max memory size, which runtime can allocate at once.
/// Substrate allocator restrict allocations bigger then 512 wasm pages at once.
/// See more information about:
/// https://github.com/paritytech/substrate/blob/cc4d5cc8654d280f03a13421669ba03632e14aa7/client/allocator/src/freeing_bump.rs#L136-L149
/// https://github.com/paritytech/substrate/blob/cc4d5cc8654d280f03a13421669ba03632e14aa7/primitives/core/src/lib.rs#L385-L388
const RUNTIME_MAX_ALLOC_SIZE: usize = 512 * 0x10000;

/// Take half from [RUNTIME_MAX_ALLOC_SIZE] in order to avoid problems with capacity overflow.
const RUNTIME_MAX_BUFF_SIZE: usize = RUNTIME_MAX_ALLOC_SIZE / 2;

/// Runtime buffer size exceed error
#[derive(
    Clone, Copy, Default, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Decode, Encode, TypeInfo,
)]
pub struct RuntimeBufferSizeError;

impl From<RuntimeBufferSizeError> for &str {
    fn from(_: RuntimeBufferSizeError) -> Self {
        "Runtime buffer size exceed"
    }
}

impl Display for RuntimeBufferSizeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str((*self).into())
    }
}

/// Buffer which size cannot be bigger then max allowed allocation size in runtime.
pub type RuntimeBuffer = LimitedVec<u8, RuntimeBufferSizeError, RUNTIME_MAX_BUFF_SIZE>;

#[cfg(test)]
mod test {
    use super::{LimitedVec, RuntimeBufferSizeError};
    use alloc::{string::String, vec, vec::Vec};
    use core::convert::{TryFrom, TryInto};

    const N: usize = 1000;
    type TestBuffer = LimitedVec<u8, RuntimeBufferSizeError, N>;
    const M: usize = 64;
    type SmallTestBuffer = LimitedVec<u8, RuntimeBufferSizeError, M>;

    #[test]
    fn test_try_from() {
        let v1 = vec![1; N];
        let v2 = vec![1; N + 1];
        let v3 = vec![1; N - 1];

        let x = TestBuffer::try_from(v1).unwrap();
        let _ = TestBuffer::try_from(v2).expect_err("Must be err because of size overflow");
        let z = TestBuffer::try_from(v3).unwrap();

        assert_eq!(x.inner().len(), N);
        assert_eq!(z.inner().len(), N - 1);
        assert_eq!(x.inner()[N / 2], 1);
        assert_eq!(z.inner()[N / 2], 1);
    }

    #[test]
    fn test_new_default() {
        let x = LimitedVec::<String, RuntimeBufferSizeError, N>::try_new_default(N).unwrap();
        assert!(
            LimitedVec::<u64, RuntimeBufferSizeError, N>::try_new_default(N + 1).is_err(),
            "Must be error because of size overflow"
        );
        let z = LimitedVec::<Vec<u8>, RuntimeBufferSizeError, N>::try_new_default(0).unwrap();

        assert_eq!(x.inner().len(), N);
        assert_eq!(z.inner().len(), 0);
        assert_eq!(x.inner()[N / 2], "");
    }

    #[test]
    fn test_prepend_works() {
        let mut buf = TestBuffer::try_from(vec![1, 2, 3, 4, 5]).unwrap();
        let prepend_buf = TestBuffer::try_from(vec![6, 7, 8]).unwrap();
        buf.try_prepend(prepend_buf).unwrap();

        assert_eq!(buf.inner(), &[6, 7, 8, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_full() {
        let mut x = TestBuffer::try_from(vec![1; N]).unwrap();
        let mut y = TestBuffer::try_from(vec![2; N / 2]).unwrap();
        let mut z = TestBuffer::try_from(vec![3; 0]).unwrap();

        x.try_push(42).unwrap_err();
        y.try_push(42).unwrap();
        z.try_push(42).unwrap();

        x.try_extend_from_slice(&[1, 2, 3]).unwrap_err();
        y.try_extend_from_slice(&[1, 2, 3]).unwrap();
        z.try_extend_from_slice(&[1, 2, 3]).unwrap();

        x.try_prepend(vec![1, 2, 3].try_into().unwrap())
            .unwrap_err();
        y.try_prepend(vec![1, 2, 3].try_into().unwrap()).unwrap();
        z.try_prepend(vec![1, 2, 3].try_into().unwrap()).unwrap();

        z.inner_mut()[0] = 0;

        assert_eq!(&z.into_vec(), &[0, 2, 3, 42, 1, 2, 3]);
        assert_eq!(TestBuffer::max_len(), N);
    }

    #[test]
    fn formatting_test() {
        use alloc::format;

        let buffer = SmallTestBuffer::try_from(b"abcdefghijklmnopqrstuvwxyz012345".to_vec())
            .expect("String is 64 bytes");

        // `Debug`/`Display`.
        assert_eq!(
            format!("{buffer:+?}"),
            "0x6162636465666768696a6b6c6d6e6f707172737475767778797a303132333435"
        );
        // `Debug`/`Display` with default precision.
        assert_eq!(
            format!("{buffer:?}"),
            "0x6162636465666768..797a303132333435"
        );
        // `Debug`/`Display` with precision 0.
        assert_eq!(format!("{buffer:.0?}"), "0x..");
        // `Debug`/`Display` with precision 1.
        assert_eq!(format!("{buffer:.1?}"), "0x61..35");
        // `Debug`/`Display` with precision 2.
        assert_eq!(format!("{buffer:.2?}"), "0x6162..3435");
        // `Debug`/`Display` with precision 4.
        assert_eq!(format!("{buffer:.4?}"), "0x61626364..32333435");
        // `Debug`/`Display` with precision 15.
        assert_eq!(
            format!("{buffer:.15?}"),
            "0x6162636465666768696a6b6c6d6e6f..72737475767778797a303132333435"
        );
        // `Debug`/`Display` with precision 30.
        assert_eq!(
            format!("{buffer:.30?}"),
            "0x6162636465666768696a6b6c6d6e6f707172737475767778797a303132333435"
        );
        // Alternate formatter with default precision.
        assert_eq!(
            format!("{buffer:#}"),
            "LimitedVec(0x6162636465666768..797a303132333435)"
        );
        // Alternate formatter with max precision.
        assert_eq!(
            format!("{buffer:+#}"),
            "LimitedVec(0x6162636465666768696a6b6c6d6e6f707172737475767778797a303132333435)"
        );
        // Alternate formatter with precision 2.
        assert_eq!(format!("{buffer:#.2}"), "LimitedVec(0x6162..3435)");
    }
}
