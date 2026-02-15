RSpec.describe 'test' do
  it 'allows change by nonzero' do
    expect { foo }.to change(Foo, :bar).by(1)
  end

  it 'allows block change by nonzero' do
    expect { foo }.to change { Foo.bar }.by(1)
  end

  it 'allows not_to change' do
    expect { foo }.not_to change(Foo, :bar)
  end

  it 'allows change by variable' do
    expect { foo }.to change(Foo, :bar).by(n)
  end

  it 'allows complex block body with by(0)' do
    expect { foo }.to change { Foo.bar + Baz.quux }.by(0)
  end
end
