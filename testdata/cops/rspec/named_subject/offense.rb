RSpec.describe User do
  subject { described_class.new }

  it "is valid" do
    expect(subject.valid?).to be(true)
           ^^^^^^^ RSpec/NamedSubject: Name your test subject if you need to reference it explicitly.
  end

  specify do
    expect(subject.valid?).to be(true)
           ^^^^^^^ RSpec/NamedSubject: Name your test subject if you need to reference it explicitly.
  end

  before(:each) do
    do_something_with(subject)
                      ^^^^^^^ RSpec/NamedSubject: Name your test subject if you need to reference it explicitly.
  end

  # subject { ... } inside a hook is a reference, not a definition
  around(:each) do |example|
    subject { Doorkeeper.configuration }
    ^^^^^^^ RSpec/NamedSubject: Name your test subject if you need to reference it explicitly.
    example.run
  end
end
