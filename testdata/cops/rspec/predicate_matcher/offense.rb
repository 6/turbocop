expect(foo.empty?).to be_truthy
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/PredicateMatcher: Prefer using `be_empty` matcher over `empty?`.
expect(foo.exist?).to be_truthy
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/PredicateMatcher: Prefer using `exist` matcher over `exist?`.
expect(foo.has_something?).to be_truthy
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/PredicateMatcher: Prefer using `have_something` matcher over `has_something?`.
