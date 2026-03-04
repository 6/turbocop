describe Foo do
  before { bar }
  ^^^^^^^^^^^^^^ RSpec/ScatteredSetup: Do not define multiple `before` hooks in the same example group (also defined on line 3).
  before { baz }
  ^^^^^^^^^^^^^^ RSpec/ScatteredSetup: Do not define multiple `before` hooks in the same example group (also defined on line 2).
end

describe Bar do
  after { cleanup_one }
  ^^^^^^^^^^^^^^^^^^^^^ RSpec/ScatteredSetup: Do not define multiple `after` hooks in the same example group (also defined on line 8).
  after { cleanup_two }
  ^^^^^^^^^^^^^^^^^^^^^ RSpec/ScatteredSetup: Do not define multiple `after` hooks in the same example group (also defined on line 7).
end

describe Baz do
  before { setup_one }
  ^^^^^^^^^^^^^^^^^^^^ RSpec/ScatteredSetup: Do not define multiple `before` hooks in the same example group (also defined on line 13).
  before { setup_two }
  ^^^^^^^^^^^^^^^^^^^^ RSpec/ScatteredSetup: Do not define multiple `before` hooks in the same example group (also defined on line 12).
  it { expect(true).to be(true) }
end

# :each, :example, and no-arg are all equivalent scopes
describe ScopeEquivalence do
  after { bar }
  ^^^^^^^^^^^^^ RSpec/ScatteredSetup: Do not define multiple `after` hooks in the same example group (also defined on lines 20, 21).
  after(:each) { baz }
  ^^^^^^^^^^^^^^^^^^^^ RSpec/ScatteredSetup: Do not define multiple `after` hooks in the same example group (also defined on lines 19, 21).
  after(:example) { qux }
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ScatteredSetup: Do not define multiple `after` hooks in the same example group (also defined on lines 19, 20).
end

# Hooks with same metadata should still be flagged
describe SameMetadata do
  before(:each, :special_case) { foo }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ScatteredSetup: Do not define multiple `before` hooks in the same example group (also defined on line 27).
  before(:example, :special_case) { bar }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ScatteredSetup: Do not define multiple `before` hooks in the same example group (also defined on line 26).
end
