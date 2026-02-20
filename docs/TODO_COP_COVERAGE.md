# Remaining Cop Coverage: Rails Schema Cops

2 cops remain to reach 100% coverage across all gems. Both require `db/schema.rb` parsing, which turbocop doesn't have yet.

**Current conformance impact: none.** Neither cop fires on any bench repo today — our no-op stubs match RuboCop's output perfectly. This is a completeness goal, not a conformance fix.

## Rails/UniqueValidationWithoutIndex

**Difficulty: Medium-Hard.** The cop itself is moderately complex (validates call matching + scope handling + polymorphic associations), but the real work is the schema loader prerequisite.

**Enabled by default** in rubocop-rails. Scoped to `**/app/models/**/*.rb`.

Detects `validates :col, uniqueness: true` without a corresponding unique database index. Without the index, race conditions can still insert duplicates and the validation SELECT is slow on large tables.

**Needs from schema:**
- Table name for the model class (derived from class name or `self.table_name =`)
- Column names being validated (including `scope:` columns)
- Whether a matching unique index exists (including expression indexes like `lower(email)`)

**Edge cases:**
- `uniqueness: false` / `uniqueness: nil` → skip
- Conditional validations (`if:`, `unless:`) → skip
- Polymorphic `belongs_to` → check both `_id` and `_type` columns
- Expression indexes → substring match on column name
- Scope with `.freeze` → unwrap frozen array

**Stub:** `src/cop/rails/unique_validation_without_index.rs` (empty `check_node`)

**Bench repos for testing:** chatwoot has explicit excludes for this cop (6 model files), suggesting it fires there. mastodon, doorkeeper, fat_free_crm, good_job all have `db/schema.rb`.

## Rails/UnusedIgnoredColumns

**Difficulty: Easy** (once the schema loader exists). The cop logic is straightforward — match `ignored_columns` assignments and check column existence.

**Disabled by default** in rubocop-rails. No bench repo enables it, so this will likely never affect conformance. Low priority.

Detects `self.ignored_columns = [:col]` where the column no longer exists in the schema. Stale `ignored_columns` entries should be cleaned up after the migration that removes the column.

**Needs from schema:**
- Table name for the model class
- Whether each referenced column exists in the table definition

**Edge cases:**
- Both `=` and `+=` assignment forms
- Both symbol and string column names
- Non-literal arrays (variable reference) → skip
- Module context (not a class) → skip

**Stub:** `src/cop/rails/unused_ignored_columns.rs` (empty `check_node`)

## Prerequisite: Schema Loader

**Difficulty: Medium.** This is the blocking prerequisite for both cops. Must be built first.

Both cops depend on the same schema infrastructure. The work breaks down into:

### 1. Schema parser

Parse `db/schema.rb` and extract structured data. The file uses a small DSL:

```ruby
ActiveRecord::Schema.define(version: 2024_01_01) do
  create_table "users" do |t|
    t.string "email", null: false
    t.string "name"
    t.index ["email"], unique: true
  end

  add_index "users", ["name"], unique: false
end
```

We need to extract:
- **Tables** — name, columns (name + type + null), inline indexes
- **Indexes** — columns or expression, unique flag
- **add_index calls** — table name + index info

This is a subset of Ruby that Prism can parse. Walk the AST for `create_table` blocks and `add_index` calls.

### 2. Table name resolution

Derive table name from ActiveRecord model class name:
- `User` → `users` (underscore + pluralize)
- `Admin::User` → `users` (last segment)
- Explicit `self.table_name = "custom_table"` overrides

Rails pluralization is complex, but a basic `s` suffix + common irregular forms covers the majority of real-world cases. RuboCop uses the same simplified approach (it doesn't load ActiveSupport's inflector).

### 3. Integration with linter

Schema data is per-project (not per-file), so it should be:
- Loaded once during config resolution
- Passed to cops that need it (new field on cop context or a shared reference)
- Optional — if `db/schema.rb` doesn't exist, these cops become no-ops

### 4. Wire up the cops

- Uncomment registrations in `src/cop/rails/mod.rs`
- Implement `check_node` using the schema data
- Add test fixtures using `# turbocop-schema:` directive or similar mechanism to provide inline schema for tests

## Suggested order

1. **Schema loader** (prerequisite) — new module `src/schema.rs`, parse with Prism, unit test against real `db/schema.rb` files from bench repos
2. **UniqueValidationWithoutIndex** — higher value since it's enabled by default; test against chatwoot
3. **UnusedIgnoredColumns** — low priority since disabled by default and no conformance impact
