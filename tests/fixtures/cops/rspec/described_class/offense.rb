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

# Class.new without a block — argument should still be flagged
describe MyClass do
  let(:subclass) { Class.new(MyClass) }
                             ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MyClass`.
end

# Struct.new without a block — argument should still be flagged
describe MyClass do
  let(:record) { Struct.new(MyClass) }
                            ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MyClass`.
end

# Non-scope-change method ending in _eval — should still flag
describe MyClass do
  before do
    safe_eval do
      MyClass.new
      ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MyClass`.
    end
  end
end

# Qualified constant (ConstantPathNode) inside singleton method — NOT a scope change
describe Foo::Bar do
  def self.build_example(klass)
    it 'uses described class' do
      expect(Foo::Bar.new).to be_truthy
             ^^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `Foo::Bar`.
    end
  end
end

# Qualified constant inside regular method call within describe
describe Chef::Decorator do
  it "#is_a? returns true" do
    expect(decorator.is_a?(Chef::Decorator)).to be true
                           ^^^^^^^^^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `Chef::Decorator`.
  end
end

# ConstantPathWriteNode — parent part of target matches described class
describe Service do
  before do
    Service::INITD_PATH = 'path'
    ^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `Service`.
  end
end

# ConstantPathWriteNode with multi-segment described class
describe Chef::Resource do
  before do
    Chef::Resource::Klz = klz
    ^^^^^^^^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `Chef::Resource`.
  end
end

# ConstantPathWriteNode with deeply qualified described class
describe Anyway::Ext::DeepDup do
  it "assigns constant" do
    Anyway::Ext::DeepDup::TestClass = klass
    ^^^^^^^^^^^^^^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `Anyway::Ext::DeepDup`.
  end
end

# Block argument (&Const) should not be treated as a scope change
describe RedactQueueProc do
  it "test" do
    instance_eval(&RedactQueueProc)
                   ^^^^^^^^^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `RedactQueueProc`.
  end
end

# Rspec (lowercase s) receiver should be recognized as top-level describe
Rspec.describe Banner do
  it 'creates' do
    Banner.new
    ^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `Banner`.
  end
end

# Rspec (lowercase s) with qualified class
Rspec.describe MebApi::DGI::Automation::Service do
  let(:service) { MebApi::DGI::Automation::Service.new }
                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `MebApi::DGI::Automation::Service`.
end

# describe inside a def method (outside describe block) should still be found
module TransportSpecMacros
  def transport_handler_eql(path, method)
    describe SockJS::Transport do
      it "test" do
        SockJS::Transport.handlers(path)
        ^^^^^^^^^^^^^^^^^ RSpec/DescribedClass: Use `described_class` instead of `SockJS::Transport`.
      end
    end
  end
end
