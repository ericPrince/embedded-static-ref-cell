# embedded-static-ref-cell

_Note: package in experimentation_

Implements a pattern for single-threaded AVR MCUs that allows a non-Sync type or a non-const function to be used
in a static variable

`StaticRefCell` is essentially a wrapper around `bare_metal::Mutex<RefCell<Option<T>>>`. The mutex uses critical
sections (via `avr_device::interrupt::free`), which are fine for single threaded use. The `Refcell` is needed so we can modify
the underlying data, and the `Option` is needed so the static variable can be initialized with `None`.

`StaticRefCell` is useful particularly when you want to reference peripheral data (i.e., pins) in interrupt service
routines. The `Peripheral` type in `avr_hal` does not implement `Send`/`Sync`, so this pattern is needed to use pins
or objects that use pins in your ISR.

## Examples

Incomplete example showing a typical pattern for `StaticRefCell` use.

This is a trivial example where the cell wraps a `bool`, but this pattern should be implemented in a similar manner for
more complex wrapped types, such as user-defined structs that use device peripherals and pins.

```rust
use avr_device;
use embedded_static_ref_cell::StaticRefCell;

// the static variable will always start out holding None
static MY_DATA: StaticRefCell<bool> = StaticRefCel::new();

#[avr_device::entry]
fn main() {
    // set the data for the StaticRefCell to non-None
    avr_device::interrupt::free(|cs| MY_DATA.init(true));

    // ...

    // interrupts are not enabled until after the static data contains a value
    // (this is generally a good pattern, but may not always be the case)
    unsafe {
        avr_device::interrupt::enable();
    }

    loop {
        // get the value in the StaticRefCell and do something with it, or
        // get the value false if the data is None
        let my_value = avr_device::interrupt::free(|cs| MY_DATA.borrow(cs, |value| value, || false));
        // ...
    }
}

#[avr_device::interrupt]
fn MY_ISR() {
    // invert the bool whenever the interrupt is triggered, and panic if the data
    // is still None (this is just shown for reference, it may be a better idea to pass
    // an empty function closure in to do nothing rather than panic if the cell isn't
    // initialized yet)
    avr_device::interrupt::free(|cs| MY_DATA.borrow_mut(cs, |value| value = !value, panic!()); // TODO: check here
}
```