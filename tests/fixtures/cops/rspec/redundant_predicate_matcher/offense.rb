expect(foo).to be_include(bar, baz)
               ^^^^^^^^^^^^^^^^^^^^ RSpec/RedundantPredicateMatcher: Use `include` instead of `be_include`.
expect(foo).to be_cover(bar, baz)
               ^^^^^^^^^^^^^^^^^^ RSpec/RedundantPredicateMatcher: Use `cover` instead of `be_cover`.
expect(foo).to be_eql(bar)
               ^^^^^^^^^^^ RSpec/RedundantPredicateMatcher: Use `eql` instead of `be_eql`.
result.should be_include("value")
              ^^^^^^^^^^^^^^^^^^ RSpec/RedundantPredicateMatcher: Use `include` instead of `be_include`.
result.should_not be_include("value")
                  ^^^^^^^^^^^^^^^^^^ RSpec/RedundantPredicateMatcher: Use `include` instead of `be_include`.

expect(repository.all).to all(be_respond_to(:connection))
                              ^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/RedundantPredicateMatcher: Use `respond_to` instead of `be_respond_to`.
