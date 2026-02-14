RSpec.describe Foo do
  it do
    subject
    expect { subject }.to not_change { Foo.count }
             ^^^^^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
  end
end

RSpec.describe Bar do
  it do
    expect { subject }.to change { Bar.count }
    expect { subject }.to not_change { Bar.count }
             ^^^^^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
  end
end

RSpec.describe Baz do
  it do
    expect(subject.a).to eq(3)
    nested_block do
      expect { on_shard(:europe) { subject } }.to not_change { Baz.count }
                                   ^^^^^^^ RSpec/RepeatedSubjectCall: Calls to subject are memoized, this block is misleading
    end
  end
end
