use std::fmt;
use std::slice;
use std::str;
use std::sync::Arc;

pub use wiggle_macro::{async_trait, from_witx};

pub use bitflags;

#[cfg(feature = "wiggle_metadata")]
pub use witx;

mod error;
mod guest_type;
mod region;

pub extern crate tracing;

pub use error::GuestError;
pub use guest_type::{GuestErrorType, GuestType, GuestTypeTransparent};
pub use region::Region;

pub mod async_trait_crate {
    pub use async_trait::*;
}

/// A trait which abstracts how to get at the region of host memory taht
/// contains guest memory.
///
/// All `GuestPtr` types will contain a handle to this trait, signifying where
/// the pointer is actually pointing into. This type will need to be implemented
/// for the host's memory storage object.
///
/// # Safety
///
/// Safety around this type is tricky, and the trait is `unsafe` since there are
/// a few contracts you need to uphold to implement this type correctly and have
/// everything else in this crate work out safely.
///
/// The most important method of this trait is the `base` method. This returns,
/// in host memory, a pointer and a length. The pointer should point to valid
/// memory for the guest to read/write for the length contiguous bytes
/// afterwards.
///
/// The region returned by `base` must not only be valid, however, but it must
/// be valid for "a period of time before the guest is reentered". This isn't
/// exactly well defined but the general idea is that `GuestMemory` is allowed
/// to change under our feet to accomodate instructions like `memory.grow` or
/// other guest modifications. Memory, however, cannot be changed if the guest
/// is not reentered or if no explicitly action is taken to modify the guest
/// memory.
///
/// This provides the guarantee that host pointers based on the return value of
/// `base` have a dynamic period for which they are valid. This time duration
/// must be "somehow nonzero in length" to allow users of `GuestMemory` and
/// `GuestPtr` to safely read and write interior data.
///
/// This type also provides methods for run-time borrow checking of references
/// into the memory. The safety of this mechanism depends on there being
/// exactly one associated tracking of borrows for a given WebAssembly memory.
/// There must be no other reads or writes of WebAssembly the memory by either
/// Rust or WebAssembly code while there are any outstanding borrows, as given
/// by `GuestMemory::has_outstanding_borrows()`.
///
/// # Using References
///
/// The [`GuestPtr::as_slice`] or [`GuestPtr::as_str`] will return smart
/// pointers [`GuestSlice`] and [`GuestStr`]. These types, which implement
/// [`std::ops::Deref`] and [`std::ops::DerefMut`], provide mutable references
/// into the memory region given by a `GuestMemory`.
///
/// These smart pointers are dynamically borrow-checked by the borrow checker
/// methods on this trait. While a `GuestSlice` or a `GuestStr` are live, the
/// [`GuestMemory::has_outstanding_borrows()`] method will always return
/// `true`. If you need to re-enter the guest or otherwise read or write to
/// the contents of a WebAssembly memory, all `GuestSlice`s and `GuestStr`s
/// for the memory must be dropped, at which point
/// `GuestMemory::has_outstanding_borrows()` will return `false`.
pub unsafe trait GuestMemory: Send + Sync {
    /// Returns the base allocation of this guest memory, located in host
    /// memory.
    ///
    /// A pointer/length pair are returned to signify where the guest memory
    /// lives in the host, and how many contiguous bytes the memory is valid for
    /// after the returned pointer.
    ///
    /// Note that there are safety guarantees about this method that
    /// implementations must uphold, and for more details see the
    /// [`GuestMemory`] documentation.
    fn base(&self) -> (*mut u8, u32);

    /// Validates a guest-relative pointer given various attributes, and returns
    /// the corresponding host pointer.
    ///
    /// * `offset` - this is the guest-relative pointer, an offset from the
    ///   base.
    /// * `align` - this is the desired alignment of the guest pointer, and if
    ///   successful the host pointer will be guaranteed to have this alignment.
    /// * `len` - this is the number of bytes, after `offset`, that the returned
    ///   pointer must be valid for.
    ///
    /// This function will guarantee that the returned pointer is in-bounds of
    /// `base`, *at this time*, for `len` bytes and has alignment `align`. If
    /// any guarantees are not upheld then an error will be returned.
    ///
    /// Note that the returned pointer is an unsafe pointer. This is not safe to
    /// use in general because guest memory can be relocated. Additionally the
    /// guest may be modifying/reading memory as well. Consult the
    /// [`GuestMemory`] documentation for safety information about using this
    /// returned pointer.
    fn validate_size_align(
        &self,
        offset: u32,
        align: usize,
        len: u32,
    ) -> Result<*mut u8, GuestError> {
        let (base_ptr, base_len) = self.base();
        let region = Region { start: offset, len };

        // Figure out our pointer to the start of memory
        let start = match (base_ptr as usize).checked_add(offset as usize) {
            Some(ptr) => ptr,
            None => return Err(GuestError::PtrOverflow),
        };
        // and use that to figure out the end pointer
        let end = match start.checked_add(len as usize) {
            Some(ptr) => ptr,
            None => return Err(GuestError::PtrOverflow),
        };
        // and then verify that our end doesn't reach past the end of our memory
        if end > (base_ptr as usize) + (base_len as usize) {
            return Err(GuestError::PtrOutOfBounds(region));
        }
        // and finally verify that the alignment is correct
        if start % align != 0 {
            return Err(GuestError::PtrNotAligned(region, align as u32));
        }
        Ok(start as *mut u8)
    }

    /// Convenience method for creating a `GuestPtr` at a particular offset.
    ///
    /// Note that `T` can be almost any type, and typically `offset` is a `u32`.
    /// The exception is slices and strings, in which case `offset` is a `(u32,
    /// u32)` of `(offset, length)`.
    fn ptr<'a, T>(&'a self, offset: T::Pointer) -> GuestPtr<'a, T>
    where
        Self: Sized,
        T: ?Sized + Pointee,
    {
        GuestPtr::new(self, offset)
    }

    /// Indicates whether any outstanding borrows are known to the
    /// `GuestMemory`. This function must be `false` in order for it to be
    /// safe to recursively call into a WebAssembly module, or to manipulate
    /// the WebAssembly memory by any other means.
    fn has_outstanding_borrows(&self) -> bool;
    /// Check if a region of linear memory is exclusively borrowed. This is called during any
    /// `GuestPtr::read` or `GuestPtr::write` operation to ensure that wiggle is not reading or
    /// writing a region of memory which Rust believes it has exclusive access to.
    fn is_mut_borrowed(&self, r: Region) -> bool;
    /// Check if a region of linear memory has any shared borrows.
    fn is_shared_borrowed(&self, r: Region) -> bool;
    /// Exclusively borrow a region of linear memory. This is used when constructing a
    /// `GuestSliceMut` or `GuestStrMut`. Those types will give Rust `&mut` access
    /// to the region of linear memory, therefore, the `GuestMemory` impl must
    /// guarantee that at most one `BorrowHandle` is issued to a given region,
    /// `GuestMemory::has_outstanding_borrows` is true for the duration of the
    /// borrow, and that `GuestMemory::is_mut_borrowed` of any overlapping region
    /// is false for the duration of the borrow.
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError>;
    /// Shared borrow a region of linear memory. This is used when constructing a
    /// `GuestSlice` or `GuestStr`. Those types will give Rust `&` (shared reference) access
    /// to the region of linear memory.
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError>;
    /// Unborrow a previously borrowed mutable region. As long as `GuestSliceMut` and
    /// `GuestStrMut` are implemented correctly, a mut `BorrowHandle` should only be
    /// unborrowed once.
    fn mut_unborrow(&self, h: BorrowHandle);
    /// Unborrow a previously borrowed shared region. As long as `GuestSlice` and
    /// `GuestStr` are implemented correctly, a shared `BorrowHandle` should only be
    /// unborrowed once.
    fn shared_unborrow(&self, h: BorrowHandle);
}

/// A handle to a borrow on linear memory. It is produced by `{mut, shared}_borrow` and
/// consumed by `{mut, shared}_unborrow`. Only the `GuestMemory` impl should ever construct
/// a `BorrowHandle` or inspect its contents.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BorrowHandle(pub usize);

// Forwarding trait implementations to the original type
unsafe impl<'a, T: ?Sized + GuestMemory> GuestMemory for &'a T {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
    fn has_outstanding_borrows(&self) -> bool {
        T::has_outstanding_borrows(self)
    }
    fn is_mut_borrowed(&self, r: Region) -> bool {
        T::is_mut_borrowed(self, r)
    }
    fn is_shared_borrowed(&self, r: Region) -> bool {
        T::is_shared_borrowed(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}

unsafe impl<'a, T: ?Sized + GuestMemory> GuestMemory for &'a mut T {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
    fn has_outstanding_borrows(&self) -> bool {
        T::has_outstanding_borrows(self)
    }
    fn is_mut_borrowed(&self, r: Region) -> bool {
        T::is_mut_borrowed(self, r)
    }
    fn is_shared_borrowed(&self, r: Region) -> bool {
        T::is_shared_borrowed(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}

unsafe impl<T: ?Sized + GuestMemory> GuestMemory for Box<T> {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
    fn has_outstanding_borrows(&self) -> bool {
        T::has_outstanding_borrows(self)
    }
    fn is_mut_borrowed(&self, r: Region) -> bool {
        T::is_mut_borrowed(self, r)
    }
    fn is_shared_borrowed(&self, r: Region) -> bool {
        T::is_shared_borrowed(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}

unsafe impl<T: ?Sized + GuestMemory> GuestMemory for Arc<T> {
    fn base(&self) -> (*mut u8, u32) {
        T::base(self)
    }
    fn has_outstanding_borrows(&self) -> bool {
        T::has_outstanding_borrows(self)
    }
    fn is_mut_borrowed(&self, r: Region) -> bool {
        T::is_mut_borrowed(self, r)
    }
    fn is_shared_borrowed(&self, r: Region) -> bool {
        T::is_shared_borrowed(self, r)
    }
    fn mut_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::mut_borrow(self, r)
    }
    fn shared_borrow(&self, r: Region) -> Result<BorrowHandle, GuestError> {
        T::shared_borrow(self, r)
    }
    fn mut_unborrow(&self, h: BorrowHandle) {
        T::mut_unborrow(self, h)
    }
    fn shared_unborrow(&self, h: BorrowHandle) {
        T::shared_unborrow(self, h)
    }
}

/// A *guest* pointer into host memory.
///
/// This type represents a pointer from the guest that points into host memory.
/// Internally a `GuestPtr` contains a handle to its original [`GuestMemory`] as
/// well as the offset into the memory that the pointer is pointing at.
///
/// Presence of a [`GuestPtr`] does not imply any form of validity. Pointers can
/// be out-of-bounds, misaligned, etc. It is safe to construct a `GuestPtr` with
/// any offset at any time. Consider a `GuestPtr<T>` roughly equivalent to `*mut
/// T`, although there are a few more safety guarantees around this type.
///
/// ## Slices and Strings
///
/// Note that the type parameter does not need to implement the `Sized` trait,
/// so you can implement types such as this:
///
/// * `GuestPtr<'_, str>` - a pointer to a guest string. Has the methods
///   [`GuestPtr::as_str_mut`], which gives a dynamically borrow-checked
///   `GuestStrMut<'_>`, which `DerefMut`s to a `&mut str`, and
///   [`GuestPtr::as_str`], which is the sharedable version of same.
/// * `GuestPtr<'_, [T]>` - a pointer to a guest array. Has methods
///   [`GuestPtr::as_slice_mut`], which gives a dynamically borrow-checked
///   `GuestSliceMut<'_, T>`, which `DerefMut`s to a `&mut [T]` and
///   [`GuestPtr::as_slice`], which is the sharedable version of same.
///
/// Unsized types such as this may have extra methods and won't have methods
/// like [`GuestPtr::read`] or [`GuestPtr::write`].
///
/// ## Type parameter and pointee
///
/// The `T` type parameter is largely intended for more static safety in Rust as
/// well as having a better handle on what we're pointing to. A `GuestPtr<T>`,
/// however, does not necessarily literally imply a guest pointer pointing to
/// type `T`. Instead the [`GuestType`] trait is a layer of abstraction where
/// `GuestPtr<T>` may actually be a pointer to `U` in guest memory, but you can
/// construct a `T` from a `U`.
///
/// For example `GuestPtr<GuestPtr<T>>` is a valid type, but this is actually
/// more equivalent to `GuestPtr<u32>` because guest pointers are always
/// 32-bits. That being said you can create a `GuestPtr<T>` from a `u32`.
///
/// Additionally `GuestPtr<MyEnum>` will actually delegate, typically, to and
/// implementation which loads the underlying data as `GuestPtr<u8>` (or
/// similar) and then the bytes loaded are validated to fit within the
/// definition of `MyEnum` before `MyEnum` is returned.
///
/// For more information see the [`GuestPtr::read`] and [`GuestPtr::write`]
/// methods. In general though be extremely careful about writing `unsafe` code
/// when working with a `GuestPtr` if you're not using one of the
/// already-attached helper methods.
pub struct GuestPtr<'a, T: ?Sized + Pointee> {
    mem: &'a (dyn GuestMemory + 'a),
    pointer: T::Pointer,
}

impl<'a, T: ?Sized + Pointee> GuestPtr<'a, T> {
    /// Creates a new `GuestPtr` from the given `mem` and `pointer` values.
    ///
    /// Note that for sized types like `u32`, `GuestPtr<T>`, etc, the `pointer`
    /// value is a `u32` offset into guest memory. For slices and strings,
    /// `pointer` is a `(u32, u32)` offset/length pair.
    pub fn new(mem: &'a (dyn GuestMemory + 'a), pointer: T::Pointer) -> GuestPtr<'a, T> {
        GuestPtr { mem, pointer }
    }

    /// Returns the offset of this pointer in guest memory.
    ///
    /// Note that for sized types this returns a `u32`, but for slices and
    /// strings it returns a `(u32, u32)` pointer/length pair.
    pub fn offset(&self) -> T::Pointer {
        self.pointer
    }

    /// Returns the guest memory that this pointer is coming from.
    pub fn mem(&self) -> &'a (dyn GuestMemory + 'a) {
        self.mem
    }

    /// Casts this `GuestPtr` type to a different type.
    ///
    /// This is a safe method which is useful for simply reinterpreting the type
    /// parameter on this `GuestPtr`. Note that this is a safe method, where
    /// again there's no guarantees about alignment, validity, in-bounds-ness,
    /// etc of the returned pointer.
    pub fn cast<U>(&self) -> GuestPtr<'a, U>
    where
        T: Pointee<Pointer = u32>,
    {
        GuestPtr::new(self.mem, self.pointer)
    }

    /// Safely read a value from this pointer.
    ///
    /// This is a fun method, and is one of the lynchpins of this
    /// implementation. The highlight here is that this is a *safe* operation,
    /// not an unsafe one like `*mut T`. This works for a few reasons:
    ///
    /// * The `unsafe` contract of the `GuestMemory` trait means that there's
    ///   always at least some backing memory for this `GuestPtr<T>`.
    ///
    /// * This does not use Rust-intrinsics to read the type `T`, but rather it
    ///   delegates to `T`'s implementation of [`GuestType`] to actually read
    ///   the underlying data. This again is a safe method, so any unsafety, if
    ///   any, must be internally documented.
    ///
    /// * Eventually what typically happens it that this bottoms out in the read
    ///   implementations for primitives types (like `i32`) which can safely be
    ///   read at any time, and then it's up to the runtime to determine what to
    ///   do with the bytes it read in a safe manner.
    ///
    /// Naturally lots of things can still go wrong, such as out-of-bounds
    /// checks, alignment checks, validity checks (e.g. for enums), etc. All of
    /// these check failures, however, are returned as a [`GuestError`] in the
    /// `Result` here, and `Ok` is only returned if all the checks passed.
    pub fn read(&self) -> Result<T, GuestError>
    where
        T: GuestType<'a>,
    {
        T::read(self)
    }

    /// Safely write a value to this pointer.
    ///
    /// This method, like [`GuestPtr::read`], is pretty crucial for the safe
    /// operation of this crate. All the same reasons apply though for why this
    /// method is safe, even eventually bottoming out in primitives like writing
    /// an `i32` which is safe to write bit patterns into memory at any time due
    /// to the guarantees of [`GuestMemory`].
    ///
    /// Like `read`, `write` can fail due to any manner of pointer checks, but
    /// any failure is returned as a [`GuestError`].
    pub fn write(&self, val: T) -> Result<(), GuestError>
    where
        T: GuestType<'a>,
    {
        T::write(self, val)
    }

    /// Performs pointer arithmetic on this pointer, moving the pointer forward
    /// `amt` slots.
    ///
    /// This will either return the resulting pointer or `Err` if the pointer
    /// arithmetic calculation would overflow around the end of the address
    /// space.
    pub fn add(&self, amt: u32) -> Result<GuestPtr<'a, T>, GuestError>
    where
        T: GuestType<'a> + Pointee<Pointer = u32>,
    {
        let offset = amt
            .checked_mul(T::guest_size())
            .and_then(|o| self.pointer.checked_add(o));
        let offset = match offset {
            Some(o) => o,
            None => return Err(GuestError::PtrOverflow),
        };
        Ok(GuestPtr::new(self.mem, offset))
    }

    /// Returns a `GuestPtr` for an array of `T`s using this pointer as the
    /// base.
    pub fn as_array(&self, elems: u32) -> GuestPtr<'a, [T]>
    where
        T: GuestType<'a> + Pointee<Pointer = u32>,
    {
        GuestPtr::new(self.mem, (self.pointer, elems))
    }
}

impl<'a, T> GuestPtr<'a, [T]> {
    /// For slices, specifically returns the relative pointer to the base of the
    /// array.
    ///
    /// This is similar to `<[T]>::as_ptr()`
    pub fn offset_base(&self) -> u32 {
        self.pointer.0
    }

    /// For slices, returns the length of the slice, in elements.
    pub fn len(&self) -> u32 {
        self.pointer.1
    }

    /// Returns an iterator over interior pointers.
    ///
    /// Each item is a `Result` indicating whether it overflowed past the end of
    /// the address space or not.
    pub fn iter<'b>(
        &'b self,
    ) -> impl ExactSizeIterator<Item = Result<GuestPtr<'a, T>, GuestError>> + 'b
    where
        T: GuestType<'a>,
    {
        let base = self.as_ptr();
        (0..self.len()).map(move |i| base.add(i))
    }

    /// Attempts to create a [`GuestSlice<'_, T>`] from this pointer, performing
    /// bounds checks and type validation. The `GuestSlice` is a smart pointer
    /// that can be used as a `&[T]` via the `Deref` trait.
    /// The region of memory backing the slice will be marked as sharedably
    /// borrowed by the [`GuestMemory`] until the `GuestSlice` is dropped.
    /// Multiple sharedable borrows of the same memory are permitted, but only
    /// one mutable borrow.
    ///
    /// This function will return a `GuestSlice` into host memory if all checks
    /// succeed (valid utf-8, valid pointers, memory is not borrowed, etc). If
    /// any checks fail then `GuestError` will be returned.
    pub fn as_slice(&self) -> Result<GuestSlice<'a, T>, GuestError>
    where
        T: GuestTypeTransparent<'a>,
    {
        let len = match self.pointer.1.checked_mul(T::guest_size()) {
            Some(l) => l,
            None => return Err(GuestError::PtrOverflow),
        };
        let ptr =
            self.mem
                .validate_size_align(self.pointer.0, T::guest_align(), len)? as *mut T;

        let borrow = self.mem.shared_borrow(Region {
            start: self.pointer.0,
            len,
        })?;

        // Validate all elements in slice.
        // SAFETY: ptr has been validated by self.mem.validate_size_align
        for offs in 0..self.pointer.1 {
            T::validate(unsafe { ptr.add(offs as usize) })?;
        }

        // SAFETY: iff there are no overlapping mut borrows it is valid to construct a &[T]
        let ptr = unsafe { slice::from_raw_parts(ptr, self.pointer.1 as usize) };

        Ok(GuestSlice {
            ptr,
            mem: self.mem,
            borrow,
        })
    }

    /// Attempts to create a [`GuestSliceMut<'_, T>`] from this pointer, performing
    /// bounds checks and type validation. The `GuestSliceMut` is a smart pointer
    /// that can be used as a `&[T]` or a `&mut [T]` via the `Deref` and `DerefMut`
    /// traits. The region of memory backing the slice will be marked as borrowed
    /// by the [`GuestMemory`] until the `GuestSlice` is dropped.
    ///
    /// This function will return a `GuestSliceMut` into host memory if all checks
    /// succeed (valid utf-8, valid pointers, memory is not borrowed, etc). If
    /// any checks fail then `GuestError` will be returned.
    pub fn as_slice_mut(&self) -> Result<GuestSliceMut<'a, T>, GuestError>
    where
        T: GuestTypeTransparent<'a>,
    {
        let len = match self.pointer.1.checked_mul(T::guest_size()) {
            Some(l) => l,
            None => return Err(GuestError::PtrOverflow),
        };
        let ptr =
            self.mem
                .validate_size_align(self.pointer.0, T::guest_align(), len)? as *mut T;

        let borrow = self.mem.mut_borrow(Region {
            start: self.pointer.0,
            len,
        })?;

        // Validate all elements in slice.
        // SAFETY: ptr has been validated by self.mem.validate_size_align
        for offs in 0..self.pointer.1 {
            T::validate(unsafe { ptr.add(offs as usize) })?;
        }

        // SAFETY: iff there are no overlapping borrows it is valid to construct a &mut [T]
        let ptr = unsafe { slice::from_raw_parts_mut(ptr, self.pointer.1 as usize) };

        Ok(GuestSliceMut {
            ptr,
            mem: self.mem,
            borrow,
        })
    }

    /// Copies the data pointed to by `slice` into this guest region.
    ///
    /// This method is a *safe* method to copy data from the host to the guest.
    /// This requires that `self` and `slice` have the same length. The pointee
    /// type `T` requires the [`GuestTypeTransparent`] trait which is an
    /// assertion that the representation on the host and on the guest is the
    /// same.
    ///
    /// # Errors
    ///
    /// Returns an error if this guest pointer is out of bounds or if the length
    /// of this guest pointer is not equal to the length of the slice provided.
    pub fn copy_from_slice(&self, slice: &[T]) -> Result<(), GuestError>
    where
        T: GuestTypeTransparent<'a> + Copy + 'a,
    {
        // bounds check ...
        let mut self_slice = self.as_slice_mut()?;
        // ... length check ...
        if self_slice.len() != slice.len() {
            return Err(GuestError::SliceLengthsDiffer);
        }
        // ... and copy!
        self_slice.copy_from_slice(slice);
        Ok(())
    }

    /// Returns a `GuestPtr` pointing to the base of the array for the interior
    /// type `T`.
    pub fn as_ptr(&self) -> GuestPtr<'a, T> {
        GuestPtr::new(self.mem, self.offset_base())
    }

    pub fn get(&self, index: u32) -> Option<GuestPtr<'a, T>>
    where
        T: GuestType<'a>,
    {
        if index < self.len() {
            Some(
                self.as_ptr()
                    .add(index)
                    .expect("just performed bounds check"),
            )
        } else {
            None
        }
    }

    pub fn get_range(&self, r: std::ops::Range<u32>) -> Option<GuestPtr<'a, [T]>>
    where
        T: GuestType<'a>,
    {
        if r.end < r.start {
            return None;
        }
        let range_length = r.end - r.start;
        if r.start <= self.len() && r.end <= self.len() {
            Some(
                self.as_ptr()
                    .add(r.start)
                    .expect("just performed bounds check")
                    .as_array(range_length),
            )
        } else {
            None
        }
    }
}

impl<'a> GuestPtr<'a, str> {
    /// For strings, returns the relative pointer to the base of the string
    /// allocation.
    pub fn offset_base(&self) -> u32 {
        self.pointer.0
    }

    /// Returns the length, in bytes, of the string.
    pub fn len(&self) -> u32 {
        self.pointer.1
    }

    /// Returns a raw pointer for the underlying slice of bytes that this
    /// pointer points to.
    pub fn as_bytes(&self) -> GuestPtr<'a, [u8]> {
        GuestPtr::new(self.mem, self.pointer)
    }

    /// Returns a pointer for the underlying slice of bytes that this
    /// pointer points to.
    pub fn as_byte_ptr(&self) -> GuestPtr<'a, [u8]> {
        GuestPtr::new(self.mem, self.pointer)
    }

    /// Attempts to create a [`GuestStr<'_>`] from this pointer, performing
    /// bounds checks and utf-8 checks. The resulting `GuestStr` can be used
    /// as a `&str` via the `Deref` trait. The region of memory backing the
    /// `str` will be marked as sharedably borrowed by the [`GuestMemory`]
    /// until the `GuestStr` is dropped.
    ///
    /// This function will return `GuestStr` into host memory if all checks
    /// succeed (valid utf-8, valid pointers, etc). If any checks fail then
    /// `GuestError` will be returned.
    pub fn as_str(&self) -> Result<GuestStr<'a>, GuestError> {
        let ptr = self
            .mem
            .validate_size_align(self.pointer.0, 1, self.pointer.1)?;

        let borrow = self.mem.shared_borrow(Region {
            start: self.pointer.0,
            len: self.pointer.1,
        })?;

        // SAFETY: iff there are no overlapping borrows it is ok to construct
        // a &mut str.
        let ptr = unsafe { slice::from_raw_parts(ptr, self.pointer.1 as usize) };
        // Validate that contents are utf-8:
        match str::from_utf8(ptr) {
            Ok(ptr) => Ok(GuestStr {
                ptr,
                mem: self.mem,
                borrow,
            }),
            Err(e) => Err(GuestError::InvalidUtf8(e)),
        }
    }

    /// Attempts to create a [`GuestStrMut<'_>`] from this pointer, performing
    /// bounds checks and utf-8 checks. The resulting `GuestStrMut` can be used
    /// as a `&str` or `&mut str` via the `Deref` and `DerefMut` traits. The
    /// region of memory backing the `str` will be marked as borrowed by the
    /// [`GuestMemory`] until the `GuestStrMut` is dropped.
    ///
    /// This function will return `GuestStrMut` into host memory if all checks
    /// succeed (valid utf-8, valid pointers, etc). If any checks fail then
    /// `GuestError` will be returned.
    pub fn as_str_mut(&self) -> Result<GuestStrMut<'a>, GuestError> {
        let ptr = self
            .mem
            .validate_size_align(self.pointer.0, 1, self.pointer.1)?;

        let borrow = self.mem.mut_borrow(Region {
            start: self.pointer.0,
            len: self.pointer.1,
        })?;

        // SAFETY: iff there are no overlapping borrows it is ok to construct
        // a &mut str.
        let ptr = unsafe { slice::from_raw_parts_mut(ptr, self.pointer.1 as usize) };
        // Validate that contents are utf-8:
        match str::from_utf8_mut(ptr) {
            Ok(ptr) => Ok(GuestStrMut {
                ptr,
                mem: self.mem,
                borrow,
            }),
            Err(e) => Err(GuestError::InvalidUtf8(e)),
        }
    }
}

impl<'a> GuestPtr<'a, [u8]> {
    /// Returns a pointer to the string represented by a `[u8]` without
    /// validating whether each u8 is a utf-8 codepoint.
    pub fn as_str_ptr(&self) -> GuestPtr<'a, str> {
        GuestPtr::new(self.mem, self.pointer)
    }
}

impl<T: ?Sized + Pointee> Clone for GuestPtr<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized + Pointee> Copy for GuestPtr<'_, T> {}

impl<T: ?Sized + Pointee> fmt::Debug for GuestPtr<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        T::debug(self.pointer, f)
    }
}

/// A smart pointer to an sharedable slice in guest memory.
/// Usable as a `&'a [T]` via [`std::ops::Deref`].
pub struct GuestSlice<'a, T> {
    ptr: &'a [T],
    mem: &'a dyn GuestMemory,
    borrow: BorrowHandle,
}

impl<'a, T> std::ops::Deref for GuestSlice<'a, T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}

impl<'a, T> Drop for GuestSlice<'a, T> {
    fn drop(&mut self) {
        self.mem.shared_unborrow(self.borrow)
    }
}

/// A smart pointer to a mutable slice in guest memory.
/// Usable as a `&'a [T]` via [`std::ops::Deref`] and as a `&'a mut [T]` via
/// [`std::ops::DerefMut`].
pub struct GuestSliceMut<'a, T> {
    ptr: &'a mut [T],
    mem: &'a dyn GuestMemory,
    borrow: BorrowHandle,
}

impl<'a, T> std::ops::Deref for GuestSliceMut<'a, T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}

impl<'a, T> std::ops::DerefMut for GuestSliceMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ptr
    }
}

impl<'a, T> Drop for GuestSliceMut<'a, T> {
    fn drop(&mut self) {
        self.mem.mut_unborrow(self.borrow)
    }
}

/// A smart pointer to an sharedable `str` in guest memory.
/// Usable as a `&'a str` via [`std::ops::Deref`].
pub struct GuestStr<'a> {
    ptr: &'a str,
    mem: &'a dyn GuestMemory,
    borrow: BorrowHandle,
}

impl<'a> std::ops::Deref for GuestStr<'a> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}

impl<'a> Drop for GuestStr<'a> {
    fn drop(&mut self) {
        self.mem.shared_unborrow(self.borrow)
    }
}

/// A smart pointer to a mutable `str` in guest memory.
/// Usable as a `&'a str` via [`std::ops::Deref`] and as a `&'a mut str` via
/// [`std::ops::DerefMut`].
pub struct GuestStrMut<'a> {
    ptr: &'a mut str,
    mem: &'a dyn GuestMemory,
    borrow: BorrowHandle,
}

impl<'a> std::ops::Deref for GuestStrMut<'a> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}

impl<'a> std::ops::DerefMut for GuestStrMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ptr
    }
}

impl<'a> Drop for GuestStrMut<'a> {
    fn drop(&mut self) {
        self.mem.mut_unborrow(self.borrow)
    }
}

mod private {
    pub trait Sealed {}
    impl<T> Sealed for T {}
    impl<T> Sealed for [T] {}
    impl Sealed for str {}
}

/// Types that can be pointed to by `GuestPtr<T>`.
///
/// In essence everything can, and the only special-case is unsized types like
/// `str` and `[T]` which have special implementations.
pub trait Pointee: private::Sealed {
    #[doc(hidden)]
    type Pointer: Copy;
    #[doc(hidden)]
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result;
}

impl<T> Pointee for T {
    type Pointer = u32;
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "*guest {:#x}", pointer)
    }
}

impl<T> Pointee for [T] {
    type Pointer = (u32, u32);
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "*guest {:#x}/{}", pointer.0, pointer.1)
    }
}

impl Pointee for str {
    type Pointer = (u32, u32);
    fn debug(pointer: Self::Pointer, f: &mut fmt::Formatter) -> fmt::Result {
        <[u8]>::debug(pointer, f)
    }
}

/// A runtime-independent way for Wiggle to terminate WebAssembly execution.
/// Functions that are marked `(@witx noreturn)` will always return a Trap.
/// Other functions that want to Trap can do so via their `UserErrorConversion`
/// trait, which transforms the user's own error type into a `Result<abierror, Trap>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trap {
    /// A Trap which indicates an i32 (posix-style) exit code. Runtimes may have a
    /// special way of dealing with this for WASI embeddings and otherwise.
    I32Exit(i32),
    /// Any other Trap is just an unstructured String, for reporting and debugging.
    String(String),
}

impl From<GuestError> for Trap {
    fn from(err: GuestError) -> Trap {
        Trap::String(err.to_string())
    }
}

pub fn run_in_dummy_executor<F: std::future::Future>(future: F) -> F::Output {
    use std::pin::Pin;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    let mut f = Pin::from(Box::new(future));
    let waker = dummy_waker();
    let mut cx = Context::from_waker(&waker);
    match f.as_mut().poll(&mut cx) {
        Poll::Ready(val) => return val,
        Poll::Pending => {
            panic!("Cannot wait on pending future: must enable wiggle \"async\" future and execute on an async Store")
        }
    }

    fn dummy_waker() -> Waker {
        return unsafe { Waker::from_raw(clone(5 as *const _)) };

        unsafe fn clone(ptr: *const ()) -> RawWaker {
            assert_eq!(ptr as usize, 5);
            const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
            RawWaker::new(ptr, &VTABLE)
        }

        unsafe fn wake(ptr: *const ()) {
            assert_eq!(ptr as usize, 5);
        }

        unsafe fn wake_by_ref(ptr: *const ()) {
            assert_eq!(ptr as usize, 5);
        }

        unsafe fn drop(ptr: *const ()) {
            assert_eq!(ptr as usize, 5);
        }
    }
}
