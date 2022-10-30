//! Implements a pattern for single-threaded AVR MCUs that allows a non-Sync type or a non-const function to be used
//! in a static variable
//!
//! `StaticRefCell` is essentially a wrapper around `avr_device::interrupt::Mutex<RefCell<Option<T>>>`. The mutex uses critical
//! sections (via `avr_device::interrupt::free`), which are fine for single threaded use. The `Refcell` is needed so we can modify
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
//! ```
//! // the static variable will always start out holding None
//! static MY_DATA: StaticRefCell<bool> = StaticRefCel::new();
//!
//! #[avr_device::entry]
//! fn main() {
//!     // set the data for the StaticRefCell to non-None
//!     avr_device::interrupt::free(|cs| MY_DATA.init(true));
//!
//!     // ...
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
//!         let my_value = avr_device::interrupt::free(|cs| MY_DATA.borrow(cs, |value| value, || false);
//!         // ...
//!     }
//! }
//!
//! #[avr_device::interrupt]
//! fn MY_ISR() {
//!     // invert the bool whenever the interrupt is triggered, and panic if the data
//!     // is still None (this is just shown for reference, it may be a better idea to pass
//!     // an empty function closure in to do nothing rather than panic if the cell isn't
//!     // initialized yet)
//!     avr_device::interrupt::free(|cs| MY_DATA.borrow_mut(cs, |value| value = !value, panic!()); // TODO: check here
//! }
//! ```

#![no_std]

use bare_metal::{CriticalSection, Mutex};
use core::cell::RefCell;

type MRCO<T> = Mutex<RefCell<Option<T>>>;

/// An object that allows for a non-Send/Sync type to be used in a static variable in an AVR MCU
///
/// See the module-level documentation for more details
pub struct StaticRefcell<T>(MRCO<T>);

impl<T> StaticRefcell<T> {
    /// Creates a new uninitialized object (stored value as None)
    pub const fn new() -> Self {
        Self(Mutex::new(RefCell::new(None)))
    }

    /// Sets the stored value for this object
    ///
    /// Requires passing in a CriticalSection, such as the one used in `avr_device::interrupt::free`
    pub fn init(&self, cs: &CriticalSection, value: T) {
        *self.0.borrow(cs).borrow_mut() = Some(value);
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
    /// Note that `avr_device::interrupt::free` and this function both propagate the return value.
    ///
    /// ```
    /// struct MyData{
    ///     data: i32
    /// }
    /// let cell: StaticRefCell<MyData> = StaticRefCell::new();
    /// cell.init(MyData{data: 1});
    /// let cell_value = avr_device::interrupt::free(|cs| cell.borrow(cs, |value| value.data, || -1));
    /// assert_eq!(cell_value, 1);
    /// ```
    pub fn borrow<F>(&self, cs: &CriticalSection, func: fn(&T) -> F, none_func: fn() -> F) -> F {
        match self.0.borrow(cs).borrow().as_ref() {
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
    /// struct MyData{
    ///     data: i32
    /// }
    /// let cell: StaticRefCell<MyData> = StaticRefCell::new();
    /// cell.init(MyData{data: 0});
    /// avr_device::interrupt::free(|cs| cell.borrow_mut(cs, |value| value.data += 1, || panic!()));
    ///
    /// let cell_value = avr_device::interrupt::free(|cs| cell.borrow(cs, |value| value, || MyData{data: -1}));
    /// assert_eq!(cell_value, MyData{data: 1});
    /// ```
    pub fn borrow_mut<F>(
        &self,
        cs: &CriticalSection,
        func: fn(&mut T) -> F,
        none_func: fn() -> F,
    ) -> F {
        match self.0.borrow(cs).borrow_mut().as_mut() {
            Some(value) => func(value),
            None => none_func(),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn it_works() {
//         let result = add(2, 2);
//         assert_eq!(result, 4);
//     }
// }
