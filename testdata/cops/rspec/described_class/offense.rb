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
