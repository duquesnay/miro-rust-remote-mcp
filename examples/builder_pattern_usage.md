# Builder Pattern Usage Guide

This document demonstrates the improved ergonomics of using builder patterns for complex Miro API operations.

## Overview

The builder pattern has been implemented for four methods that previously had many parameters:
- `create_sticky_note` (6 parameters)
- `create_shape` (9 parameters)
- `create_text` (6 parameters)
- `create_connector` (8 parameters)

## Benefits

✅ **More Readable** - Fluent API makes intent clear
✅ **Self-Documenting** - Named methods vs positional parameters
✅ **Optional Parameters** - Explicitly opt-in to optional features
✅ **Type Safety** - Required parameters enforced at compile time
✅ **Default Values** - Sensible defaults for common use cases

## Sticky Note Example

### Before (Positional Parameters)

```rust
// Hard to remember parameter order
// Unclear what None values mean
let note = client.create_sticky_note(
    "board-123",
    "Hello World".to_string(),
    100.0,
    200.0,
    "light_yellow".to_string(),
    Some("frame-456".to_string()),
).await?;
```

### After (Builder Pattern)

```rust
// Clear, readable, self-documenting
let note = client.sticky_note("board-123", "Hello World", 100.0, 200.0)
    .color("light_yellow")
    .parent_id("frame-456")
    .build(&client)
    .await?;

// Or with defaults (light_yellow color, no parent)
let simple_note = client.sticky_note("board-123", "Quick note", 0.0, 0.0)
    .build(&client)
    .await?;
```

## Shape Example

### Before (9 Parameters!)

```rust
// Very hard to understand what's happening
// Easy to mix up width/height/x/y
let shape = client.create_shape(
    "board-123",
    "rectangle".to_string(),
    "light_blue".to_string(),
    0.0,
    100.0,
    200.0,
    150.0,
    Some("<p>Content</p>".to_string()),
    Some("frame-456".to_string()),
).await?;
```

### After (Builder Pattern)

```rust
// Intent is crystal clear
// Named dimensions prevent confusion
let shape = client.shape("board-123", "rectangle", 0.0, 100.0, 200.0, 150.0)
    .fill_color("light_blue")
    .content("<p>Content</p>")
    .parent_id("frame-456")
    .build(&client)
    .await?;

// Simple shape with defaults
let simple_shape = client.shape("board-123", "circle", 50.0, 50.0, 100.0, 100.0)
    .build(&client)
    .await?;
```

## Text Example

### Before

```rust
let text = client.create_text(
    "board-123",
    "Some text".to_string(),
    0.0,
    0.0,
    300.0,
    None,
).await?;
```

### After (Builder Pattern)

```rust
// Clean and readable
let text = client.text("board-123", "Some text", 0.0, 0.0, 300.0)
    .build(&client)
    .await?;

// With parent frame
let framed_text = client.text("board-123", "Some text", 0.0, 0.0, 300.0)
    .parent_id("frame-789")
    .build(&client)
    .await?;
```

## Connector Example

### Before (8 Parameters)

```rust
// Which parameter is stroke_color vs stroke_width?
// What do all these Nones mean?
let connector = client.create_connector(
    "board-123",
    "item-1".to_string(),
    "item-2".to_string(),
    Some("blue".to_string()),
    Some(2.5),
    Some("none".to_string()),
    Some("arrow".to_string()),
    None,
).await?;
```

### After (Builder Pattern)

```rust
// Each property is named and clear
let connector = client.connector("board-123", "item-1", "item-2")
    .stroke_color("blue")
    .stroke_width(2.5)
    .start_cap("none")
    .end_cap("arrow")
    .build(&client)
    .await?;

// Simple connector with defaults
let simple_connector = client.connector("board-123", "item-1", "item-2")
    .build(&client)
    .await?;

// With caption
let labeled_connector = client.connector("board-123", "item-1", "item-2")
    .stroke_color("red")
    .end_cap("arrow")
    .caption("depends on", Some(0.5))
    .build(&client)
    .await?;

// Multiple captions
let detailed_connector = client.connector("board-123", "item-1", "item-2")
    .caption("start label", Some(0.2))
    .caption("end label", Some(0.8))
    .build(&client)
    .await?;
```

## Direct Builder Construction

You can also construct builders directly without going through `MiroClient`:

```rust
use miro_mcp_server::miro::builders::*;

// Direct builder construction
let note = StickyNoteBuilder::new("board-123", "Hello", 0.0, 0.0)
    .color("yellow")
    .build(&client)
    .await?;

let shape = ShapeBuilder::new("board-123", "rectangle", 0.0, 0.0, 100.0, 100.0)
    .fill_color("blue")
    .build(&client)
    .await?;
```

## Backward Compatibility

**All original methods remain unchanged** - the builder pattern is purely additive:

```rust
// Old code still works exactly as before
let note = client.create_sticky_note(
    "board-123",
    "Hello".to_string(),
    0.0,
    0.0,
    "yellow".to_string(),
    None,
).await?;

// New code uses builders for better ergonomics
let note = client.sticky_note("board-123", "Hello", 0.0, 0.0)
    .color("yellow")
    .build(&client)
    .await?;
```

## Migration Recommendations

### When to Use Builders

✅ Use builders for **new code** - better ergonomics and clarity
✅ Use builders when **optional parameters** are needed
✅ Use builders for **better code reviews** - intent is explicit

### When Original Methods Are OK

✅ Existing code works fine - no need to refactor
✅ Simple cases with no optional parameters
✅ Scripts or one-off code where verbosity isn't critical

## Pattern Summary

All builders follow this consistent pattern:

```rust
client.[method](board_id, required_params...)
    .optional_param1(value)
    .optional_param2(value)
    .build(&client)
    .await?
```

**Key Points:**
- Required parameters in constructor
- Optional parameters as builder methods
- `build(&client)` executes the API call
- Fluent API allows chaining

## Testing

The builder pattern has comprehensive unit tests ensuring:
- ✅ All parameters correctly passed through
- ✅ Default values work as expected
- ✅ Optional parameters can be omitted
- ✅ Multiple captions can be added (connectors)
- ✅ All builders implement expected trait bounds

Run tests: `cargo test miro::builders`
