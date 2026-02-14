describe MyClass do
  let(:foo) { [] }

  it { expect(foo).to be_empty }

  it 'uses local variables' do
    bar = compute_something
    expect(bar).to eq(42)
  end
end
