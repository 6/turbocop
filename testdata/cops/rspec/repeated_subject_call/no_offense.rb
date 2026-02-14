RSpec.describe Foo do
  it do
    expect(subject.a).to eq(3)
    expect(subject.b).to eq(4)
  end
end

RSpec.describe Bar do
  it do
    expect { subject }.to change { Bar.count }
    expect(subject.b).to eq(4)
  end
end
