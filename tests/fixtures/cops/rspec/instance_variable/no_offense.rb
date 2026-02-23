describe MyClass do
  let(:foo) { [] }

  it { expect(foo).to be_empty }

  it 'uses local variables' do
    bar = compute_something
    expect(bar).to eq(42)
  end

  # Instance variables inside method definitions are OK
  def helper_method
    @internal_state = 42
    @other_var
  end

  def compute
    @result ||= expensive_call
  end
end

# Instance variables inside Class.new / Struct.new blocks are OK
describe Integration do
  let(:klass) do
    Class.new do
      def initialize
        @name = 'test'
      end
    end
  end

  it { expect(klass.new).to be_valid }
end

# Instance variables inside RSpec.configure are OK (not an example group)
RSpec.configure do |config|
  config.before(:suite) do
    @shared_resource = create_resource
  end
end

# Instance variables inside custom matchers are OK
RSpec::Matchers.define :have_attr do
  match do |actual|
    @stored = actual.attr
    @stored.present?
  end
end

# Instance variable WRITES in before blocks are not flagged (only reads are)
describe WritesInBefore do
  before do
    @user = create(:user)
    @problem = create(:problem)
  end

  # These writes are fine — the cop only flags reads
end

# Instance variable writes in before(:all) / before(:context)
describe SharedSetup do
  before(:all) do
    @app = create(:app)
    @err = create(:err)
  end
end

# Instance variable writes directly in example group are not flagged
describe DirectWrites do
  before { @foo = [] }
  before { @bar ||= compute }
  before { @count += 1 }
  before { @flag &&= false }
end
