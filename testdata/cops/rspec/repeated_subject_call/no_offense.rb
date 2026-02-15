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

# Chained subject calls are not flagged (subject is a receiver)
RSpec.describe Baz do
  it do
    expect(subject.reblogs_count).to eq(1)
    expect { subject.destroy }.to_not raise_error
  end
end
