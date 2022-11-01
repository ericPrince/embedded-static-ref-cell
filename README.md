# embedded-static-ref-cell

_Note: package in experimentation_

Builds upon `critical-section` and implements a pattern for single-threaded AVR MCUs that allows a non-Sync type or a
non-const function to be used in a static variable

`StaticRefCell` is essentially a wrapper around `critical_section::Mutex<RefCell<Option<T>>>`. The mutex uses critical
sections (via `critical_section::with`), which are fine for single threaded use. The `Refcell` is needed so we can modify
the underlying data, and the `Option` is needed so the static variable can be initialized with `None`.

`StaticRefCell` is useful particularly when you want to reference peripheral data (i.e., pins) in interrupt service
routines. The `Peripheral` type in `avr_hal` does not implement `Send`/`Sync`, so this pattern is needed to use pins
or objects that use pins in your ISR.

## Examples

Incomplete example showing a typical pattern for `StaticRefCell` use.

This is a trivial example where the cell wraps a `bool`, but this pattern should be implemented in a similar manner for
more complex wrapped types, such as user-defined structs that use device peripherals and pins.

```rust
#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use avr_device;
use critical_section;
use embedded_static_ref_cell::StaticRefCell;

// the static variable will always start out holding None
static MY_DATA: StaticRefCell<bool> = StaticRefCel::new();

#[avr_device::entry]
fn main() {
    // set the data for the StaticRefCell to non-None
    critical_section::with(|cs| MY_DATA.init(cs, true));

    // set interrupt registers to enable your interrupt, etc...

    // interrupts are not enabled until after the static data contains a value
    // (this is generally a good pattern, but may not always be the case)
    unsafe {
        avr_device::interrupt::enable();
    }

    loop {
        // get the value in the StaticRefCell and do something with it, or
        // get the value false if the data is None
        let my_value = critical_section::with(|cs| MY_DATA.borrow(cs, |value| value, || false));
        // ...
    }
}

#[avr_device::interrupt]
fn MY_ISR_NAME() {
    // invert the bool whenever the interrupt is triggered, and panic if the data
    // is still None (this is just shown for reference, it may be a better idea to pass
    // an empty function closure in to do nothing rather than panic if the cell isn't
    // initialized yet)
    critical_section::with(|cs| MY_DATA.borrow_mut(cs, |value| value = !value, panic!())); // TODO: check here
}
```
