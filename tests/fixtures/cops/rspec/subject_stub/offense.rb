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

# do...end block on receive chain followed by chain method
describe Processor do
  subject { described_class.new }

  it 'detects do...end with chain' do
    expect(subject).to receive(:process).and_wrap_original do |original, item|
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
      original.call(item)
    end.at_least(:once)
    subject.call
  end
end

# Explicit parens on .to(receive(...)).and_return(...)
describe Handler do
  subject { described_class.new }

  it 'detects explicit parens with chain' do
    allow(subject).to(receive(:load_resource)).and_return(resource)
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
    subject.run
  end
end

# do...end block on receive chain followed by .and_return
describe Forwarder do
  subject(:forwarder) { described_class.new }

  it 'detects do...end with and_return' do
    expect(forwarder).to receive(:spawn) do |*args|
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
      expect(args).to start_with('ssh')
    end.and_return(9999)
    forwarder.forward
  end
end

# Subject stub inside a nested block on an unrelated call chain
# (e.g., expect(Thread).to receive(:new) do |&block| expect(subject_name).to receive(:method); block.call end)
describe Worker do
  subject(:worker) { described_class.new }

  let(:fake_thread) { double }

  it 'detects subject stub inside nested block' do
    expect(Thread).to receive(:new) do |&block|
      expect(worker).to receive(:shutdown!)
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
      block.call
    end.and_return(fake_thread)
    worker.run
  end
end

describe Browser do
  describe '#open' do
    context 'when the server is not running' do
      it 'yields an error message' do
        expect(subject).to receive(:fork) do |&block|
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
          expect(subject).to receive(:server_running?).and_return false
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
          block.call
        end
      end
    end

    context 'when the server is running but the launch command fails' do
      it 'yields an error message' do
        expect(subject).to receive(:fork) do |&block|
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
          expect(subject).to receive(:server_running?).and_return true
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
          expect(subject).to receive(:open!).and_return false
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
          block.call
        end
      end
    end

    context 'when the server is running and the launch command succeeds' do
      it 'yields nil' do
        expect(subject).to receive(:fork) do |&block|
        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
          expect(subject).to receive(:server_running?).and_return true
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
          expect(subject).to receive(:open!).and_return true
          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/SubjectStub: Do not stub methods of the object under test.
          block.call
        end
      end
    end
  end
end
