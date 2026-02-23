describe Foo do
  subject(:foo) { described_class.new }

  before do
    allow(foo).to receive(:bar).and_return(baz)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
  end

  it 'uses expect twice' do
    expect(foo.bar).to eq(baz)
  end
end

describe Bar do
  subject(:bar) { described_class.new }

  before do
    expect(bar).to receive(:baz)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
  end

  it 'tests bar' do
    expect(bar.baz).to eq(true)
  end
end

describe Baz do
  subject { described_class.new }

  it 'stubs subject' do
    expect(subject).to receive(:qux)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
  end
end
