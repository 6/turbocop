# Lua Custom Cops

turbocop supports user-defined cops written in Lua. This lets organizations enforce custom rules — ban specific API patterns, require certain code structures, enforce naming conventions — without forking turbocop or writing Rust.

## Why Lua

- **Tiny**: ~200KB runtime, embedded directly in the turbocop binary
- **Fast**: LuaJIT-class performance, microsecond FFI overhead per call
- **Proven**: The most widely used embedded scripting language (Neovim, Redis, nginx, game engines)
- **Simple**: Fits in your head in a day. Cop definitions are typically 15–40 lines.

## Quick Start

Create `.turbocop/cops/ban_recursive_open_struct.lua`:

```lua
return {
  name = "Custom/BanRecursiveOpenStruct",
  severity = "warning",
  node_types = {"constant_read_node", "constant_path_node"},

  check_node = function(node, source, config)
    if node:source_text() == "RecursiveOpenStruct" then
      -- Skip class/module definitions (e.g., class RecursiveOpenStruct)
      local parent = node:parent()
      if parent and (parent:type() == "class_node" or parent:type() == "module_node") then
        return nil
      end
      return {line = node:line(), column = node:column(), message = "Avoid RecursiveOpenStruct."}
    end
  end,
}
```

Run turbocop normally — custom cops are auto-discovered:

```
$ turbocop .
app/models/user.rb:12:5: W: Custom/BanRecursiveOpenStruct: Avoid RecursiveOpenStruct.
```

## Loading

Custom cops are auto-discovered from the `.turbocop/cops/` directory in your project root. Every `.lua` file in that directory is loaded as a cop.

```
my-project/
  .turbocop/
    cops/
      ban_execute.lua
      require_strict_struct.lua
      check_sdk_paths.lua
  .rubocop.yml
  app/
```

No configuration needed. If `.turbocop/cops/` doesn't exist, no custom cops are loaded.

### Gem distribution

To distribute custom cops as a gem, ship a `.turbocop/cops/` directory inside the gem. turbocop discovers it from the gem's install path (resolved via `bundle info --path`).

## Cop Definition Format

A Lua cop file returns a table with:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Fully qualified name, e.g. `"Custom/MyRule"` |
| `severity` | string | no | `"convention"` (default), `"warning"`, or `"error"` |
| `node_types` | array of strings | no | AST node types to receive. Omit for all nodes. |
| `needs_parent` | boolean | no | Set `true` to enable `node:parent()`. Default `false`. |
| `check_lines` | function | no | Line-based check (no AST needed) |
| `check_source` | function | no | Full-source check (raw bytes + AST available) |
| `check_node` | function | no | Called per AST node during tree walk |

At least one of `check_lines`, `check_source`, or `check_node` must be defined.

### Return values

Each callback returns either:
- `nil` — no offense
- A single diagnostic table: `{line = N, column = N, message = "..."}`
- An array of diagnostic tables: `{{line = 1, column = 0, message = "..."}, ...}`

`line` is 1-indexed. `column` is 0-indexed (byte offset).

## Lua API Reference

### Node

Wraps a Prism AST node. Available in `check_node` and `check_source` callbacks.

```lua
-- Identity
node:type()           -- "call_node", "string_node", "class_node", etc.
node:line()           -- 1-indexed line number
node:column()         -- 0-indexed byte column
node:source_text()    -- raw source text of this node

-- Tree navigation
node:parent()         -- parent node or nil (requires needs_parent = true)
node:children()       -- array of all child nodes

-- CallNode (method calls: foo.bar(x), foo&.bar, bar(x))
node:name()           -- method name string (e.g., "bar")
node:receiver()       -- receiver node or nil (nil for bare method calls)
node:arguments()      -- array of argument nodes
node:call_operator()  -- "." or "&." or nil (bare call)
node:block()          -- attached block node or nil

-- ConstantReadNode / ConstantPathNode
node:const_name()     -- constant name (e.g., "Foo" or "Foo::Bar::Baz")

-- StringNode / SymbolNode / InterpolatedStringNode
node:value()          -- unescaped string/symbol value

-- ClassNode / ModuleNode
node:const()          -- class/module name node
node:body()           -- body statements node or nil
node:superclass()     -- superclass node or nil (ClassNode only)

-- BlockNode
node:parameters()     -- block parameters node or nil
node:body()           -- block body statements node or nil

-- IfNode / UnlessNode
node:predicate()      -- condition node
node:consequent()     -- then-branch node or nil
node:alternate()      -- else-branch node or nil

-- DefNode (method definition)
node:def_name()       -- method name string
node:parameters()     -- parameter list node or nil
node:body()           -- method body node or nil

-- HashNode / KeywordHashNode
node:pairs()          -- array of AssocNode children

-- AssocNode (key-value pair in hash)
node:key()            -- key node
node:assoc_value()    -- value node

-- ArrayNode
node:elements()       -- array of element nodes
```

### Source

Wraps the source file. Available in all callbacks.

```lua
source:path()                  -- file path string
source:text()                  -- full source text as string
source:lines()                 -- iterator: for line_num, line_text in source:lines() do ... end
source:is_code(line, column)   -- false if position is inside a comment or string
source:root()                  -- AST root node (for manual traversal in check_source)
```

### Config

Wraps the cop's configuration from `.rubocop.yml`. Available in all callbacks.

```lua
config:get_str("EnforcedStyle", "default")   -- string with default
config:get_bool("AllowInHeredoc", false)      -- boolean with default
config:get_number("Max", 120)                 -- number with default
config:get_list("AllowedMethods")             -- array of strings, or nil
```

Config keys correspond to the cop's section in `.rubocop.yml`:

```yaml
Custom/MyRule:
  Enabled: true
  EnforcedStyle: strict
  AllowedMethods:
    - initialize
    - call
```

### Helpers

```lua
-- Walk all descendants of a node
node:descendants()    -- iterator: for child in node:descendants() do ... end

-- Walk descendants filtered by type
node:find(type)       -- iterator: for n in node:find("call_node") do ... end
```

## Examples

### Simple: Ban a method call

Flag any call to `ActiveRecord::Base.transaction`:

```lua
return {
  name = "Custom/NoBaseTransaction",
  severity = "warning",
  node_types = {"call_node"},

  check_node = function(node, source, config)
    if node:name() ~= "transaction" then return end

    local recv = node:receiver()
    if recv and recv:type() == "constant_path_node"
           and recv:const_name() == "ActiveRecord::Base" then
      return {
        line = node:line(),
        column = node:column(),
        message = "Use ApplicationRecord.transaction instead of ActiveRecord::Base.transaction.",
      }
    end
  end,
}
```

### Medium: Check a send chain

Flag `*.connection.execute(...)` calls:

```lua
return {
  name = "Custom/NoDirectExecute",
  severity = "warning",
  node_types = {"call_node"},

  check_node = function(node, source, config)
    if node:name() ~= "execute" then return end

    local recv = node:receiver()
    if recv and recv:type() == "call_node" and recv:name() == "connection" then
      return {
        line = node:line(),
        column = node:column(),
        message = "Avoid direct connection.execute. Use ActiveRecord query methods instead.",
      }
    end
  end,
}
```

### Medium: Enforce a code structure

Ensure `Dry::Struct` subclasses call `schema.strict`:

```lua
return {
  name = "Custom/StrictDryStruct",
  severity = "warning",
  node_types = {"class_node"},

  check_node = function(node, source, config)
    -- Check if this class inherits from Dry::Struct
    local superclass = node:superclass()
    if not superclass then return end
    if superclass:type() ~= "constant_path_node" then return end
    if superclass:const_name() ~= "Dry::Struct" then return end

    -- Search class body for schema.strict call
    local body = node:body()
    if not body then
      return {line = node:line(), column = node:column(),
              message = "Dry::Struct subclass must call schema.strict."}
    end

    for call in body:find("call_node") do
      if call:name() == "strict" then
        local recv = call:receiver()
        if recv and recv:type() == "call_node" and recv:name() == "schema" then
          return nil  -- Found it
        end
      end
    end

    return {line = node:line(), column = node:column(),
            message = "Dry::Struct subclass must call schema.strict."}
  end,
}
```

### Complex: Recursive chain traversal

Detect incorrect mock patterns — `allow(SomeService).to receive(:graphql)` should use `receive_request` instead:

```lua
return {
  name = "Custom/GraphqlMockStyle",
  severity = "warning",
  needs_parent = true,
  node_types = {"call_node"},

  check_node = function(node, source, config)
    -- Look for receive(:graphql) calls
    if node:name() ~= "receive" then return end

    local args = node:arguments()
    if #args == 0 then return end
    if args[1]:type() ~= "symbol_node" or args[1]:value() ~= "graphql" then return end

    -- Walk up the chain to find allow(...) with a qualifying constant
    local function find_allow_target(n)
      if not n then return nil end
      if n:type() == "call_node" and n:name() == "allow" then
        local allow_args = n:arguments()
        if #allow_args > 0 then return allow_args[1] end
      end
      return find_allow_target(n:receiver() or n:parent())
    end

    local target = find_allow_target(node:parent())
    if not target then return end

    -- Check if the allow target is a service constant we care about
    local target_name = target:source_text()
    local patterns = config:get_list("ServicePatterns") or {"Service"}
    for _, pattern in ipairs(patterns) do
      if target_name:find(pattern) then
        return {
          line = node:line(),
          column = node:column(),
          message = "Use receive_request(:graphql) instead of receive(:graphql) for "
                    .. target_name .. ". This validates query structure.",
        }
      end
    end
  end,
}
```

### Line-based: Check for trailing whitespace in comments

```lua
return {
  name = "Custom/CommentTrailingWhitespace",
  severity = "convention",

  check_lines = function(source, config)
    local diagnostics = {}
    for line_num, line in source:lines() do
      -- Only check comment lines
      local comment_start = line:find("#")
      if comment_start and line:match("%s+$") then
        local trailing_start = #line - #line:match("%s+$")
        table.insert(diagnostics, {
          line = line_num,
          column = trailing_start,
          message = "Trailing whitespace in comment.",
        })
      end
    end
    return diagnostics
  end,
}
```

## Node Type Reference

These strings are valid for the `node_types` filter array. They correspond to Prism AST node types.

Common types used in custom cops:

| Node type | Ruby syntax |
|-----------|-------------|
| `"call_node"` | `foo.bar(x)`, `bar(x)`, `foo&.bar` |
| `"constant_read_node"` | `Foo` |
| `"constant_path_node"` | `Foo::Bar::Baz` |
| `"string_node"` | `"hello"`, `'hello'` |
| `"interpolated_string_node"` | `"hello #{name}"` |
| `"symbol_node"` | `:foo` |
| `"class_node"` | `class Foo ... end` |
| `"module_node"` | `module Foo ... end` |
| `"def_node"` | `def foo ... end` |
| `"block_node"` | `foo { ... }`, `foo do ... end` |
| `"if_node"` | `if ... end` |
| `"hash_node"` | `{a: 1, b: 2}` |
| `"keyword_hash_node"` | `foo(a: 1, b: 2)` (keyword args) |
| `"array_node"` | `[1, 2, 3]` |
| `"integer_node"` | `42` |
| `"local_variable_write_node"` | `x = 1` |
| `"instance_variable_read_node"` | `@foo` |
| `"global_variable_read_node"` | `$foo` |

The full list of 151 node types matches Prism's AST specification.

## Testing

Each Lua cop can include inline test cases using the same `^` annotation format as turbocop's built-in cops. Add a `tests` table with `offense` and `no_offense` keys:

```lua
return {
  name = "Custom/NoBaseTransaction",
  severity = "warning",
  node_types = {"call_node"},

  check_node = function(node, source, config)
    if node:name() ~= "transaction" then return end
    local recv = node:receiver()
    if recv and recv:type() == "constant_path_node"
           and recv:const_name() == "ActiveRecord::Base" then
      return {
        line = node:line(),
        column = node:column(),
        message = "Use ApplicationRecord.transaction instead of ActiveRecord::Base.transaction.",
      }
    end
  end,

  tests = {
    offense = [[
ActiveRecord::Base.transaction do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Custom/NoBaseTransaction: Use ApplicationRecord.transaction instead of ActiveRecord::Base.transaction.
  update!(status: :active)
end
    ]],
    no_offense = [[
ApplicationRecord.transaction do
  update!(status: :active)
end

User.transaction do
  create!(name: "test")
end
    ]],
  },
}
```

Run tests for all custom cops:

```
$ turbocop test
Custom/NoBaseTransaction: OK (1 offense, 2 no-offense cases)
Custom/NoDirectExecute: OK (1 offense, 3 no-offense cases)
Custom/StrictDryStruct: OK (2 offenses, 1 no-offense case)
3 cops, 3 passed, 0 failed
```

Or test a single cop:

```
$ turbocop test .turbocop/cops/no_base_transaction.lua
Custom/NoBaseTransaction: OK (1 offense, 2 no-offense cases)
```

### How it works

- `offense` blocks use `^` annotations to mark expected offenses, identical to turbocop's built-in fixture format: the caret line appears after the offending source line, with `^` characters spanning the offense range, followed by `CopName: message`
- `no_offense` blocks must produce zero diagnostics
- `turbocop test` strips annotations to get clean source, runs the cop, and compares actual vs expected diagnostics
- Multiple `offense` blocks can be provided as an array: `offense = {[[...]], [[...]]}`

## Configuration in .rubocop.yml

Custom cops are configured like any other cop:

```yaml
Custom/NoBaseTransaction:
  Enabled: true
  Severity: error

Custom/GraphqlMockStyle:
  Enabled: true
  ServicePatterns:
    - Service
    - Client
  Exclude:
    - "spec/support/**/*"
```

`Enabled`, `Severity`, `Include`, `Exclude` work identically to built-in cops.

---

## Implementation Notes

This section covers how Lua cops integrate with turbocop's Rust internals.

### LuaCop struct

```rust
pub struct LuaCop {
    name: &'static str,             // Leaked from Lua string at load time
    severity: Severity,
    node_types: &'static [u8],      // Compiled from Lua string array
    needs_parent: bool,
    lua_source: Vec<u8>,            // Compiled Lua bytecode
    has_check_lines: bool,
    has_check_source: bool,
    has_check_node: bool,
}
```

`LuaCop` implements `Cop` (which requires `Send + Sync`). The struct holds only metadata and bytecode — no Lua VM references. The actual Lua function calls happen through thread-local VMs.

### Thread-local Lua VMs

`mlua::Lua` is `!Send`. Each rayon worker thread gets its own VM:

```rust
thread_local! {
    static LUA_VM: RefCell<Option<mlua::Lua>> = RefCell::new(None);
}
```

On first use in a thread, the VM is created and all cop bytecode is loaded. Subsequent calls reuse the existing VM. This means:
- No locking or synchronization
- Each thread pays the VM creation cost once
- Cop bytecode is loaded N times (once per thread), but bytecode is small

### UserData bindings

Prism AST nodes, SourceFile, and CopConfig are exposed to Lua as `mlua::UserData`:

- **LuaNode**: Wraps a serialized node representation (since Prism nodes are lifetime-bound to `ParseResult`). Contains node type tag, location, and serialized child references.
- **LuaSource**: Wraps a reference to `SourceFile` data (path, content bytes, line offsets).
- **LuaConfig**: Wraps a reference to `CopConfig` options HashMap.

### Dispatch integration

Lua cops participate in the same `BatchedCopWalker` dispatch table as Rust cops:

1. `LuaCop::interested_node_types()` returns the compiled `node_types` array
2. The walker's dispatch table routes matching nodes to the Lua cop
3. `LuaCop::check_node()` serializes the node, enters the thread-local Lua VM, calls the Lua function, and converts returned tables to `Diagnostic` values

### Parent tracking

When any cop (Lua or Rust) sets `needs_parent = true`, the walker maintains a parent stack during traversal. `node:parent()` reads from this stack. When no cop needs parents, the stack is not allocated.

### Performance expectations

- **Lua VM creation**: ~1ms per thread (one-time cost)
- **Per-node FFI call**: ~1–5 microseconds (dominated by node serialization)
- **Typical cop**: 10–50ms per 10,000 nodes (comparable to a moderately complex Rust cop)
- **Recommendation**: Keep custom cops under ~20 for negligible impact on total lint time
