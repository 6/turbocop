expect(foo).to be_empty
expect(foo).to have_something
expect(foo.something?).to eq "something"
expect(foo.something).to be(true)
expect(foo.has_something).to be(true)
expect(foo).not_to be_empty

# Bare predicate calls without a receiver should not be flagged
# (they are locally-defined helper methods, not predicates on an object)
expect(enabled?('Layout/DotPosition')).to be(false)
expect(enabled?('Layout/EndOfLine')).to be(false)
expect(cop_enabled?(cop_class)).to be true
expect(valid?).to be_truthy
expect(something?).to be_falsey
