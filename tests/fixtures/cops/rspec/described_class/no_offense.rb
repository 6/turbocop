describe MyClass do
  subject { described_class.new }

  it 'works' do
    expect(described_class).to be_a(Class)
  end
end

describe "MyClass" do
  subject { "MyClass" }
end

describe MyClass do
end

# Class reference inside a def method — described_class not available there
RSpec.describe RuboCop::Cop::Utils::FormatString do
  def format_sequences(string)
    RuboCop::Cop::Utils::FormatString.new(string).format_sequences
  end
end

# context with a class arg does NOT set described_class
describe SomeApp do
  context SomeApp::Stream do
    it 'works' do
      expect(out).to be_a(SomeApp::Stream)
    end
  end
end

# module inside describe is a scope change — class reference there is fine
describe MyClass do
  module MyHelper
    MyClass.do_something
  end
end

# Non-matching namespace: bare MyClass is NOT the same as MyNamespace::MyClass
describe MyNamespace::MyClass do
  subject { ::MyClass }
  let(:foo) { MyClass }
end

# Non-matching namespace in usage
module UnrelatedNamespace
  describe MyClass do
    subject { MyNamespace::MyClass }
  end
end

# Non-matching namespace inside module
module MyNamespace
  describe MyClass do
    subject { ::MyClass }
  end
end

# OnlyStaticConstants: true (default) — don't flag constants used as namespace prefix
describe MyClass do
  subject { MyClass::FOO }
end

describe MyClass do
  subject { MyClass::Subclass }
end

# Class.new / Module.new / Struct.new / Data.define are scope changes
describe MyClass do
  Class.new  { foo = MyClass }
  Module.new { bar = MyClass }
  Struct.new { lol = MyClass }
  Data.define { dat = MyClass }

  def method
    include MyClass
  end

  class OtherClass
    include MyClass
  end

  module MyModule
    include MyClass
  end
end

# *_eval and *_exec blocks are scope changes
RSpec.describe Foo do
  before do
    stub_const('Dummy', Class.new).class_eval do
      Foo.new
    end

    stub_const('Dummy', Class.new).module_eval do
      Foo.new
    end

    stub_const('Dummy', Class.new).instance_eval do
      Foo.new
    end

    stub_const('Dummy', Class.new).class_exec do
      Foo.new
    end

    stub_const('Dummy', Class.new).module_exec do
      Foo.new
    end
  end
end

# Parenthesized namespaces — not a const path, should not be flagged
describe MyClass do
  subject { (MyNamespace)::MyClass }
end

# described_class as part of a constant should not be flagged
module SomeGem
  describe VERSION do
    it 'returns proper version string' do
      expect(described_class::STRING).to eq('1.1.1')
    end
  end
end

# describe without a class arg — no described_class available
describe do
  before do
    MyNamespace::MyClass.new
  end
end

# Accessing constants from variables in nested namespace
module Foo
  describe MyClass do
    let(:foo) { SomeClass }
    let(:baz) { foo::CONST }
  end
end

# Local variable is part of the namespace
describe Broken do
  [Foo, Bar].each do |klass|
    describe klass::Baz.name do
      it { }
    end
  end
end

# Innermost describe sets described_class — outer class is not flagged
describe MyClass do
  describe MyClass::Foo do
    let(:foo) { MyClass }
  end
end
