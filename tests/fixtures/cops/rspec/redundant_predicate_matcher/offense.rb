expect(foo).to be_include(bar, baz)
               ^^^^^^^^^^^^^^^^^^^^ RSpec/RedundantPredicateMatcher: Use `include` instead of `be_include`.
expect(foo).to be_cover(bar, baz)
               ^^^^^^^^^^^^^^^^^^ RSpec/RedundantPredicateMatcher: Use `cover` instead of `be_cover`.
expect(foo).to be_eql(bar)
               ^^^^^^^^^^^ RSpec/RedundantPredicateMatcher: Use `eql` instead of `be_eql`.
