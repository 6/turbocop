Foo.left_joins(:foo).where(foos: { id: nil }).where(bar: "bar")
    ^^^^^^^^^^^^^^^^ Rails/WhereMissing: Use `where.missing(:foo)` instead of `left_joins(:foo).where(foos: { id: nil })`.

Foo.left_outer_joins(:foo).where(foos: { id: nil }).where(bar: "bar")
    ^^^^^^^^^^^^^^^^^^^^^^ Rails/WhereMissing: Use `where.missing(:foo)` instead of `left_outer_joins(:foo).where(foos: { id: nil })`.

Foo.where(foos: { id: nil }).left_joins(:foo).where(bar: "bar")
                             ^^^^^^^^^^^^^^^^ Rails/WhereMissing: Use `where.missing(:foo)` instead of `left_joins(:foo).where(foos: { id: nil })`.
