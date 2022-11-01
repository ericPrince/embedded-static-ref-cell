//! Builds upon `critical-section` and implements a pattern for single-threaded AVR MCUs that allows a non-Sync type or a
//! non-const function to be used in a static variable
//!
//! `StaticRefCell` is essentially a wrapper around `critical_section::Mutex<RefCell<Option<T>>>`. The mutex uses critical
//! sections (via `critical_section::with`), which are fine for single threaded use. The `Refcell` is needed so we can modify
//! the underlying data, and the `Option` is needed so the static variable can be initialized with `None`.
//!
//! `StaticRefCell` is useful particularly when you want to reference peripheral data (i.e., pins) in interrupt service
//! routines. The `Peripheral` type in `avr_hal` does not implement `Send`/`Sync`, so this pattern is needed to use pins
//! or objects that use pins in your ISR.
//!
//! # Examples
//!
//! Incomplete example showing a typical pattern for `StaticRefCell` use.
//!
//! This is a trivial example where the cell wraps a `bool`, but this pattern should be implemented in a similar manner for
//! more complex wrapped types, such as user-defined structs that use device peripherals and pins.
//!
//! ```ignore
//! #![no_std]
//! #![no_main]
//! #![feature(abi_avr_interrupt)]
//!
//! use avr_device;
//! use critical_section;
//! use embedded_static_ref_cell::StaticRefCell;
//!
//! // the static variable will always start out holding None
//! static MY_DATA: StaticRefCell<bool> = StaticRefCel::new();
//!
//! #[avr_device::entry]
//! fn main() {
//!     // set the data for the StaticRefCell to non-None
//!     critical_section::with(|cs| MY_DATA.init(cs, true));
//!
//!     // set interrupt registers to enable your interrupt, etc...
//!
//!     // interrupts are not enabled until after the static data contains a value
//!     // (this is generally a good pattern, but may not always be the case)
//!     unsafe {
//!         avr_device::interrupt::enable();
//!     }
//!
//!     loop {
//!         // get the value in the StaticRefCell and do something with it, or
//!         // get the value false if the data is None
//!         let my_value = critical_section::with(|cs| MY_DATA.borrow(cs, |value| value, || false));
//!         // ...
//!     }
//! }
//!
//! #[avr_device::interrupt]
//! fn MY_ISR_NAME() {
//!     // invert the bool whenever the interrupt is triggered, and panic if the data
//!     // is still None (this is just shown for reference, it may be a better idea to pass
//!     // an empty function closure in to do nothing rather than panic if the cell isn't
//!     // initialized yet)
//!     critical_section::with(|cs| MY_DATA.borrow_mut(cs, |value| value = !value, panic!())); // TODO: check here
//! }
//! ```

#![no_std]

use core::cell::RefCell;
use critical_section::{CriticalSection, Mutex};

type MRCO<T> = Mutex<RefCell<Option<T>>>;

/// An object that allows for a non-Send/Sync type to be used safely in a static variable
///
/// See the module-level documentation for more details
pub struct StaticRefCell<T>(MRCO<T>);

impl<T> StaticRefCell<T> {
    /// Creates a new uninitialized object (stored value as None)
    pub const fn new() -> Self {
        Self(Mutex::new(RefCell::new(None)))
    }

    /// Sets the stored value for this object
    ///
    /// Requires passing in a CriticalSection, such as the one used in `critical_section::with`
    pub fn init(&self, cs: CriticalSection, value: T) {
        *self.0.borrow_ref_mut(cs) = Some(value);
    }

    /// Passes an immutable reference to the data stored by this object in `func` and returns the result,
    /// or returns the result of `none_func` if the stored data is still None
    ///
    /// In cases where this function is used to get data out of the stored object, consider using
    /// `none_func` to return a default value, or potentially use `|| panic!()` for `none_func` if
    /// you are certain the code is never supposed to reach this case.
    ///
    /// # Examples
    ///
    /// Copy a value from the cell
    ///
    /// Note that `critical_section::with` and this function both propagate the return value.
    ///
    /// ```
    /// # use embedded_static_ref_cell::StaticRefCell;
    /// #
    /// #[derive(Debug)]
    /// struct MyData{
    ///     data: i32
    /// }
    /// let cell: StaticRefCell<MyData> = StaticRefCell::new();
    /// critical_section::with(|cs| cell.init(cs, MyData{data: 1}));
    ///
    /// let cell_value = critical_section::with(|cs| cell.borrow(cs, |value| value.data, || -1));
    /// assert_eq!(cell_value, 1);
    /// ```
    pub fn borrow<F>(&self, cs: CriticalSection, func: fn(&T) -> F, none_func: fn() -> F) -> F {
        match self.0.borrow_ref(cs).as_ref() {
            Some(value) => func(value),
            None => none_func(),
        }
    }

    /// Passes a mutable reference to the data stored by this object in `func` and returns the result,
    /// or returns the result of `none_func` if the stored data is still None
    ///
    /// # Examples
    ///
    /// Update the value in the cell
    ///
    /// ```
    /// # use embedded_static_ref_cell::StaticRefCell;
    /// #
    /// #[derive(Debug, PartialEq, Copy, Clone)]
    /// struct MyData{
    ///     data: i32
    /// }
    /// let cell: StaticRefCell<MyData> = StaticRefCell::new();
    /// critical_section::with(|cs| cell.init(cs, MyData{data: 1}));
    ///
    /// critical_section::with(|cs| cell.borrow_mut(cs, |value| value.data += 1, || panic!()));
    ///
    /// let cell_value: MyData = critical_section::with(|cs| cell.borrow(cs, |value| value.clone(), || MyData{data: -1}));
    /// assert_eq!(cell_value, MyData{data: 2});
    /// ```
    pub fn borrow_mut<F>(
        &self,
        cs: CriticalSection,
        func: fn(&mut T) -> F,
        none_func: fn() -> F,
    ) -> F {
        match self.0.borrow_ref_mut(cs).as_mut() {
            Some(value) => func(value),
            None => none_func(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use critical_section;

    // note: test uses a critical section implementation using critical-section's std feature
    #[test]
    fn it_works() {
        #[derive(Debug)]
        struct MyData {
            data: i32,
        }

        let my_data: StaticRefCell<MyData> = StaticRefCell::new();

        // initialize data
        critical_section::with(|cs| my_data.init(cs, MyData { data: 1 }));

        // update value
        critical_section::with(|cs| my_data.borrow_mut(cs, |value| value.data = 2, || {}));

        // get updated value
        let my_value = critical_section::with(|cs| my_data.borrow(cs, |value| value.data, || 0));
        assert_eq!(my_value, 2);
    }
}
