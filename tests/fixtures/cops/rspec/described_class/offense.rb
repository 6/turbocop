describe MyClass do
  subject { MyClass.do_something }
            ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MyClass`.

  before { MyClass.do_something }
           ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MyClass`.

  it 'creates instance' do
    MyClass.new
    ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MyClass`.
  end
end

# Deeply nested reference
RSpec.describe Merger do
  describe '#initialize' do
    it 'creates' do
      Merger.new(problem)
      ^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `Merger`.
    end
  end
end

# Class reference in let block
RSpec.describe Clearer do
  let(:clearer) do
    Clearer.new
    ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `Clearer`.
  end
end

# describe wrapped in a module (e.g., module Pod)
module Wrapper
  describe Target do
    it 'creates' do
      Target.new
      ^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `Target`.
    end
  end
end
