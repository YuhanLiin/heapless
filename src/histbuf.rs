use core::mem::MaybeUninit;
use core::ptr;
use core::slice;

/// A "history buffer", similar to a write-only ring buffer of fixed length.
///
/// This buffer keeps a fixed number of elements.  On write, the oldest element
/// is overwritten. Thus, the buffer is useful to keep a history of values with
/// some desired depth, and for example calculate a rolling average.
///
/// # Examples
/// ```
/// use heapless::HistoryBuffer;
///
/// // Initialize a new buffer with 8 elements.
/// let mut buf = HistoryBuffer::<_, 8>::new();
///
/// // Starts with no data
/// assert_eq!(buf.recent(), None);
///
/// buf.write(3);
/// buf.write(5);
/// buf.extend(&[4, 4]);
///
/// // The most recent written element is a four.
/// assert_eq!(buf.recent(), Some(&4));
///
/// // To access all elements in an unspecified order, use `as_slice()`.
/// for el in buf.as_slice() { println!("{:?}", el); }
///
/// // Now we can prepare an average of all values, which comes out to 4.
/// let avg = buf.as_slice().iter().sum::<usize>() / buf.len();
/// assert_eq!(avg, 4);
/// ```
pub struct HistoryBuffer<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    write_at: usize,
    filled: bool,
}

impl<T, const N: usize> HistoryBuffer<T, N> {
    const INIT: MaybeUninit<T> = MaybeUninit::uninit();

    /// Constructs a new history buffer.
    ///
    /// The construction of a `HistoryBuffer` works in `const` contexts.
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::HistoryBuffer;
    ///
    /// // Allocate a 16-element buffer on the stack
    /// let x: HistoryBuffer<u8, 16> = HistoryBuffer::new();
    /// assert_eq!(x.len(), 0);
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Self {
            data: [Self::INIT; N],
            write_at: 0,
            filled: false,
        }
    }

    /// Clears the buffer, replacing every element with the default value of
    /// type `T`.
    pub fn clear(&mut self) {
        *self = Self::new();
    }
}

impl<T, const N: usize> HistoryBuffer<T, N>
where
    T: Copy + Clone,
{
    /// Constructs a new history buffer, where every element is the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::HistoryBuffer;
    ///
    /// // Allocate a 16-element buffer on the stack
    /// let mut x: HistoryBuffer<u8, 16> = HistoryBuffer::new_with(4);
    /// // All elements are four
    /// assert_eq!(x.as_slice(), [4; 16]);
    /// ```
    #[inline]
    pub fn new_with(t: T) -> Self {
        Self {
            data: [MaybeUninit::new(t); N],
            write_at: 0,
            filled: true,
        }
    }

    /// Clears the buffer, replacing every element with the given value.
    pub fn clear_with(&mut self, t: T) {
        *self = Self::new_with(t);
    }
}

impl<T, const N: usize> HistoryBuffer<T, N> {
    /// Returns the current fill level of the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        if self.filled {
            N
        } else {
            self.write_at
        }
    }

    /// Returns the capacity of the buffer, which is the length of the
    /// underlying backing array.
    #[inline]
    pub fn capacity(&self) -> usize {
        N
    }

    /// Writes an element to the buffer, overwriting the oldest value.
    pub fn write(&mut self, t: T) {
        if self.filled {
            // Drop the old before we overwrite it.
            unsafe { ptr::drop_in_place(self.data[self.write_at].as_mut_ptr()) }
        }
        self.data[self.write_at] = MaybeUninit::new(t);

        self.write_at += 1;
        if self.write_at == self.capacity() {
            self.write_at = 0;
            self.filled = true;
        }
    }

    /// Clones and writes all elements in a slice to the buffer.
    ///
    /// If the slice is longer than the buffer, only the last `self.len()`
    /// elements will actually be stored.
    pub fn extend_from_slice(&mut self, other: &[T])
    where
        T: Clone,
    {
        for item in other {
            self.write(item.clone());
        }
    }

    /// Returns a reference to the most recently written value.
    ///
    /// # Examples
    ///
    /// ```
    /// use heapless::HistoryBuffer;
    ///
    /// let mut x: HistoryBuffer<u8, 16> = HistoryBuffer::new();
    /// x.write(4);
    /// x.write(10);
    /// assert_eq!(x.recent(), Some(&10));
    /// ```
    pub fn recent(&self) -> Option<&T> {
        if self.write_at == 0 {
            if self.filled {
                Some(unsafe { &*self.data[self.capacity() - 1].as_ptr() })
            } else {
                None
            }
        } else {
            Some(unsafe { &*self.data[self.write_at - 1].as_ptr() })
        }
    }

    /// Returns the array slice backing the buffer, without keeping track
    /// of the write position. Therefore, the element order is unspecified.
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.data.as_ptr() as *const _, self.len()) }
    }
}

impl<T, const N: usize> Extend<T> for HistoryBuffer<T, N> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for item in iter.into_iter() {
            self.write(item);
        }
    }
}

impl<'a, T, const N: usize> Extend<&'a T> for HistoryBuffer<T, N>
where
    T: 'a + Clone,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = &'a T>,
    {
        self.extend(iter.into_iter().cloned())
    }
}

impl<T, const N: usize> Drop for HistoryBuffer<T, N> {
    fn drop(&mut self) {
        unsafe {
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(
                self.data.as_mut_ptr() as *mut T,
                self.len(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::HistoryBuffer;

    #[test]
    fn new() {
        let x: HistoryBuffer<u8, 4> = HistoryBuffer::new_with(1);
        assert_eq!(x.len(), 4);
        assert_eq!(x.as_slice(), [1; 4]);

        let x: HistoryBuffer<u8, 4> = HistoryBuffer::new();
        assert_eq!(x.as_slice(), []);
    }

    #[test]
    fn write() {
        let mut x: HistoryBuffer<u8, 4> = HistoryBuffer::new();
        x.write(1);
        x.write(4);
        assert_eq!(x.as_slice(), [1, 4]);

        x.write(5);
        x.write(6);
        x.write(10);
        assert_eq!(x.as_slice(), [10, 4, 5, 6]);

        x.extend([11, 12].iter());
        assert_eq!(x.as_slice(), [10, 11, 12, 6]);
    }

    #[test]
    fn clear() {
        let mut x: HistoryBuffer<u8, 4> = HistoryBuffer::new_with(1);
        x.clear();
        assert_eq!(x.as_slice(), []);

        let mut x: HistoryBuffer<u8, 4> = HistoryBuffer::new();
        x.clear_with(1);
        assert_eq!(x.as_slice(), [1; 4]);
    }

    #[test]
    fn recent() {
        let mut x: HistoryBuffer<u8, 4> = HistoryBuffer::new();
        assert_eq!(x.recent(), None);

        x.write(1);
        x.write(4);
        assert_eq!(x.recent(), Some(&4));

        x.write(5);
        x.write(6);
        x.write(10);
        assert_eq!(x.recent(), Some(&10));
    }

    #[test]
    fn as_slice() {
        let mut x: HistoryBuffer<u8, 4> = HistoryBuffer::new();

        assert_eq!(x.as_slice(), []);

        x.extend([1, 2, 3, 4, 5].iter());

        assert_eq!(x.as_slice(), [5, 2, 3, 4]);
    }
}
