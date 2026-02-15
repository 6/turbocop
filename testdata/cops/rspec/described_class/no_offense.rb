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
