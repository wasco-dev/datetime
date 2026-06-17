# Datetime
This is a Rust WebAssembly component that provides datetime utility functionality.

## Functionality

### Now
- now: Get the current timestamp in UTC.

### Timezone
- change-timezone: Convert a timestamp to a different timezone using any valid [ISO time zone designator format](https://en.wikipedia.org/wiki/ISO_8601#Time_zone_designators).

### Offset
- offset-datetime: Offset a timestamp by a given amount of seconds, minutes, hours, days, weeks, months, or years.
- offset-datetime-in-business-days: Offset a timestamp by a given amount, skipping weekends for units smaller than weeks.

## Building
You can build this component by running the following command in the project in your terminal:
```Bash
cargo build --target=wasm32-wasip2 --release
```

## Interfacing
To use this WebAssembly component in your own WebAssembly component, simply import this interface into your component like so:
```WIT
world your-world {
    import wasco-dev:datetime@0.1.0/datetime;

    // Your world definition.
}
```
