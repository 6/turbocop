expect(foo).to be_empty
expect(foo).to have_something
expect(foo.something?).to eq "something"
expect(foo.something).to be(true)
expect(foo.has_something).to be(true)
expect(foo).not_to be_empty
