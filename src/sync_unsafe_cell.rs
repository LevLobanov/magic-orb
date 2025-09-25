use std::cell::UnsafeCell;

/// [`UnsafeCell`], but [`Sync`].
///
/// This is just an `UnsafeCell`, except it implements `Sync`
/// if `T` implements `Sync`.
///
/// `UnsafeCell` doesn't implement `Sync`, to prevent accidental mis-use.
/// You can use `SyncUnsafeCell` instead of `UnsafeCell` to allow it to be
/// shared between threads, if that's intentional.
/// Providing proper synchronization is still the task of the user,
/// making this type just as unsafe to use.
///
/// See [`UnsafeCell`] for details.
#[repr(transparent)]
// #[rustc_diagnostic_item = "SyncUnsafeCell"]
// #[rustc_pub_transparent]
#[derive(Debug)]
pub struct SyncUnsafeCell<T: ?Sized> {
    value: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Sync> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    /// Constructs a new instance of `SyncUnsafeCell` which will wrap the specified value.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self { value: UnsafeCell::new(value) }
    }
}

impl<T: ?Sized> SyncUnsafeCell<T> {
    /// Gets a mutable pointer to the wrapped value.
    ///
    /// This can be cast to a pointer of any kind.
    /// Ensure that the access is unique (no active references, mutable or not)
    /// when casting to `&mut T`, and ensure that there are no mutations
    /// or mutable aliases going on when casting to `&T`
    #[inline]
    // #[rustc_as_ptr]
    // #[rustc_never_returns_null_ptr]
    pub const fn get(&self) -> *mut T {
        self.value.get()
    }
}