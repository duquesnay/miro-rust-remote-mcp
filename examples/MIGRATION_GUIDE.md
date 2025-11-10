# Migration Guide: Builder Pattern

## Overview

Version 0.2.0 introduces builder patterns for complex methods while maintaining **100% backward compatibility**.

## What Changed?

### New Features Added

✅ Four new builders: `StickyNoteBuilder`, `ShapeBuilder`, `TextBuilder`, `ConnectorBuilder`
✅ Convenience methods on `MiroClient`: `sticky_note()`, `shape()`, `text()`, `connector()`
✅ Fluent API for better ergonomics

### What Stayed the Same

✅ All existing methods work exactly as before
✅ No breaking changes to any APIs
✅ All tests pass (62 tests)
✅ Zero clippy warnings

## Should You Migrate?

### No Need to Migrate If...

- ❌ Your code is working fine
- ❌ You're short on time
- ❌ Code is in maintenance mode only

**Existing code will continue to work indefinitely.**

### Consider Migrating If...

- ✅ You're actively developing new features
- ✅ Code readability is important (code reviews, new team members)
- ✅ You use many optional parameters
- ✅ You want self-documenting code

## Migration Examples

### 1. Sticky Note Migration

**Before:**
```rust
let note = client.create_sticky_note(
    board_id,
    "Content".to_string(),
    100.0,
    200.0,
    "light_yellow".to_string(),
    Some("frame-123".to_string()),
).await?;
```

**After:**
```rust
let note = client.sticky_note(board_id, "Content", 100.0, 200.0)
    .color("light_yellow")
    .parent_id("frame-123")
    .build(&client)
    .await?;
```

**Changes:**
- Replace `.to_string()` with direct `&str` (builders accept `impl Into<String>`)
- Optional parameters use named methods
- Call `.build(&client).await?` at the end

---

### 2. Shape Migration

**Before:**
```rust
let shape = client.create_shape(
    board_id,
    "rectangle".to_string(),
    "blue".to_string(),
    x, y, width, height,
    None,  // content
    None,  // parent_id
).await?;
```

**After:**
```rust
let shape = client.shape(board_id, "rectangle", x, y, width, height)
    .fill_color("blue")
    .build(&client)
    .await?;
```

**Simplification:**
- Removed `None` parameters (defaults handle it)
- Named `fill_color` makes intent clear
- Fewer lines of code

---

### 3. Connector with Styling

**Before:**
```rust
let connector = client.create_connector(
    board_id,
    start_id.to_string(),
    end_id.to_string(),
    Some("blue".to_string()),
    Some(2.5),
    Some("none".to_string()),
    Some("arrow".to_string()),
    None,
).await?;
```

**After:**
```rust
let connector = client.connector(board_id, start_id, end_id)
    .stroke_color("blue")
    .stroke_width(2.5)
    .start_cap("none")
    .end_cap("arrow")
    .build(&client)
    .await?;
```

**Benefits:**
- Each property is self-documenting
- Can't mix up stroke_color vs stroke_width
- Easy to add/remove properties

---

### 4. Connector with Captions

**Before:**
```rust
let captions = vec![Caption {
    content: "depends on".to_string(),
    position: Some(0.5),
}];

let connector = client.create_connector(
    board_id,
    start_id.to_string(),
    end_id.to_string(),
    None, None, None, None,
    Some(captions),
).await?;
```

**After:**
```rust
let connector = client.connector(board_id, start_id, end_id)
    .caption("depends on", Some(0.5))
    .build(&client)
    .await?;
```

**Simplification:**
- No manual `Caption` construction
- Inline caption definition
- Much more readable

---

## Step-by-Step Migration Process

### Phase 1: Learn the Pattern (5 minutes)

1. Read the [builder_pattern_usage.md](./builder_pattern_usage.md) examples
2. Understand the basic pattern: `client.method().options().build(&client).await?`

### Phase 2: Migrate New Code (Ongoing)

When writing **new code**, use builders:

```rust
// ✅ Use this for new code
let note = client.sticky_note(board_id, content, x, y)
    .color("yellow")
    .build(&client)
    .await?;
```

### Phase 3: Opportunistic Migration (Optional)

When **touching existing code** for other reasons, consider migrating:

```rust
// If you're already changing this function...
// Consider switching to builders for better clarity

// Old
let note = client.create_sticky_note(/* 6 params */).await?;

// New
let note = client.sticky_note(board_id, content, x, y)
    .color("yellow")
    .build(&client)
    .await?;
```

### Phase 4: Systematic Migration (Optional)

If you want to **fully migrate** a codebase:

1. Search for `create_sticky_note` calls
2. Replace with builder pattern
3. Test thoroughly
4. Commit changes per method type

**Time estimate:** ~15 minutes per 50 call sites

---

## Common Patterns

### Pattern 1: Simple Cases (No Optional Params)

**Before:**
```rust
client.create_text(board_id, content.to_string(), x, y, width, None).await?
```

**After:**
```rust
client.text(board_id, content, x, y, width).build(&client).await?
```

### Pattern 2: One Optional Param

**Before:**
```rust
client.create_sticky_note(
    board_id, content.to_string(), x, y,
    "yellow".to_string(),
    Some(parent_id.to_string()),
).await?
```

**After:**
```rust
client.sticky_note(board_id, content, x, y)
    .color("yellow")
    .parent_id(parent_id)
    .build(&client)
    .await?
```

### Pattern 3: Many Optional Params

**Before:**
```rust
// 8 parameters, hard to read
client.create_connector(
    board_id,
    start.to_string(), end.to_string(),
    Some("blue".to_string()), Some(2.5),
    Some("none".to_string()), Some("arrow".to_string()),
    Some(captions),
).await?
```

**After:**
```rust
// Self-documenting, one property per line
client.connector(board_id, start, end)
    .stroke_color("blue")
    .stroke_width(2.5)
    .start_cap("none")
    .end_cap("arrow")
    .caption("label", Some(0.5))
    .build(&client)
    .await?
```

---

## Type Conversion Helpers

Builders accept `impl Into<String>` for string parameters:

```rust
// All of these work:
.color("yellow")                    // &str
.color(String::from("yellow"))      // String
.color(format!("color_{}", idx))    // String from format!
```

No need for explicit `.to_string()` calls!

---

## Testing After Migration

### Unit Tests
```bash
cargo test --lib
```

### Integration Tests (if applicable)
```bash
cargo test --test integration_tests
```

### Clippy Validation
```bash
cargo clippy -- -D warnings
```

---

## Rollback Plan

If you encounter issues after migration:

1. **Revert is safe** - just use old methods again
2. **Mix and match** - old and new APIs coexist
3. **No database/API changes** - purely code-level change

```rust
// Rollback: just switch back to original method
let note = client.create_sticky_note(
    board_id,
    content.to_string(),
    x, y,
    "yellow".to_string(),
    None,
).await?;
```

---

## FAQ

### Q: Do I have to migrate existing code?

**A:** No. Existing code continues to work unchanged.

### Q: What's the performance difference?

**A:** Zero. Builders are compile-time only - they generate the same API calls.

### Q: Can I mix old and new APIs?

**A:** Yes. Use whichever is clearer in each situation.

### Q: What if I prefer the old API?

**A:** That's fine! Both APIs are supported indefinitely.

### Q: Are there any breaking changes?

**A:** No. This is a purely additive change.

### Q: How do I know if my code needs updating?

**A:** If it compiles and tests pass, you're good. Migration is optional.

---

## Support

Questions or issues with migration?

1. Check [builder_pattern_usage.md](./builder_pattern_usage.md) for examples
2. Review this guide's common patterns section
3. Run tests after each migration step
4. Commit frequently for easy rollback

---

## Summary

✅ **No action required** - existing code works as-is
✅ **Opt-in improvement** - use builders for new code
✅ **Better ergonomics** - clearer, more maintainable code
✅ **Zero risk** - fully backward compatible
✅ **Well tested** - 62 tests, zero clippy warnings
