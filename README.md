# Typarena

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/voxell-tech/typarena#license)
[![Crates.io](https://img.shields.io/crates/v/typarena.svg)](https://crates.io/crates/typarena)
[![Downloads](https://img.shields.io/crates/d/typarena.svg)](https://crates.io/crates/typarena)
[![Docs](https://docs.rs/typarena/badge.svg)](https://docs.rs/typarena/latest/typarena/)
[![CI](https://github.com/voxell-tech/typarena/workflows/CI/badge.svg)](https://github.com/voxell-tech/typarena/actions)
[![Discord](https://img.shields.io/discord/442334985471655946.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/Mhnyp6VYEQ)

Type-keyed arena storage with stable per-type columns.

A `no_std` typing system to store multiple types in one place with
safety guarantees. Each type gets its own column, keyed by its
`TypeId`, that stays at a stable index for the lifetime of the store.
Callers can cache that index to skip the hash lookup on later access.

It offers two heterogeneous stores:

- `TypeTable`: a key-addressed map. You supply the key, and each
  value type lives in its own column under it.
- `TypePool`: an append-only store. Each insert hands back a compact
  `PoolKey` that reaches the value later with no hash lookup.

## `TypeTable`

```rust
use typarena::type_table::TypeTable;

#[derive(Debug, PartialEq)]
struct Position(f32, f32);
struct Health(u32);

// A table keyed by entity id, storing many value types.
let mut table = TypeTable::<u32>::new();

// Different types live under the same key, each in its own column.
table.insert(0, Position(1.0, 2.0));
table.insert(0, Health(100));

assert_eq!(table.get::<Position>(&0), Some(&Position(1.0, 2.0)));

// Cache a column id to skip the `TypeId` hash lookup on hot paths.
let col = table.type_column::<Position>().unwrap();
assert_eq!(
    table.get_by_column::<Position>(col, &0),
    Some(&Position(1.0, 2.0)),
);
```

## `TypePool`

```rust
use typarena::type_pool::TypePool;

let mut pool = TypePool::new();

// Insert values of any type; each insert returns a `PoolKey`.
let speed = pool.insert(3.14_f32);
let label = pool.insert("hello");

assert_eq!(pool.get::<f32>(&speed), Some(&3.14));
assert_eq!(pool.get::<&str>(&label), Some(&"hello"));

// The handle reaches the value directly, with no hash lookup.
assert_eq!(pool.remove::<f32>(&speed), Some(3.14));
assert_eq!(pool.get::<f32>(&speed), None);
```

## Join the community!

You can join us on the [Voxell discord server](https://discord.gg/Mhnyp6VYEQ).

## License

`typarena` is dual-licensed under either:

- MIT License ([LICENSE-MIT](/LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](/LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.
