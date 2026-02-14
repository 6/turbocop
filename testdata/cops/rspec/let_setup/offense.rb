describe Foo do
  let!(:foo) { bar }
  ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.

  it 'does not use foo' do
    expect(baz).to eq(qux)
  end
end

describe Foo do
  context 'when something special happens' do
    let!(:foo) { bar }
    ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.

    it 'does not use foo' do
      expect(baz).to eq(qux)
    end
  end

  it 'references some other foo' do
    foo
  end
end

describe Foo do
  let!(:bar) { baz }
  ^^^^^^^^^^ RSpec/LetSetup: Do not use `let!` to setup objects not referenced in tests.
end
