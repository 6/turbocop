x.dig(:foo).dig(:bar)
  ^^^^^^^^^^^^^^^^^^^ Style/DigChain: Use `dig` with multiple parameters instead of chaining.

x.dig(:foo, :bar).dig(:baz, :quux)
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DigChain: Use `dig` with multiple parameters instead of chaining.

x.y.z.dig(:foo).dig(:bar).dig(:baz)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/DigChain: Use `dig` with multiple parameters instead of chaining.
