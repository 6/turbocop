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

# include/extend/prepend are intentional module inclusions, not flagged
describe MyModule do
  controller(ApplicationController) do
    include MyModule
  end
end

describe MyModule do
  extend MyModule
  prepend MyModule
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
