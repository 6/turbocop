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
