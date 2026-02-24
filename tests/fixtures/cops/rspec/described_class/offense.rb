describe MyClass do
  include MyClass
          ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MyClass`.

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

# Fully qualified described class name should be flagged
describe MyNamespace::MyClass do
  subject { MyNamespace::MyClass }
            ^^^^^^^^^^^^^^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MyNamespace::MyClass`.
end

# Module wrapping: fully qualified name should match described class
module MyNamespace
  describe MyClass do
    subject { MyNamespace::MyClass }
              ^^^^^^^^^^^^^^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MyNamespace::MyClass`.
  end
end

# Deeply nested namespace resolution
module A
  class B::C
    module D
      describe E do
        subject { A::B::C::D::E }
                  ^^^^^^^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `A::B::C::D::E`.
        let(:one) { B::C::D::E }
                    ^^^^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `B::C::D::E`.
        let(:two) { C::D::E }
                    ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `C::D::E`.
        let(:six) { D::E }
                    ^^^^ RSpec/DescribedClass: Use `described_class` instead of `D::E`.
        let(:ten) { E }
                    ^ RSpec/DescribedClass: Use `described_class` instead of `E`.
      end
    end
  end
end
