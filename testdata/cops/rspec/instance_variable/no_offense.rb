describe MyClass do
  let(:foo) { [] }

  it { expect(foo).to be_empty }

  it 'uses local variables' do
    bar = compute_something
    expect(bar).to eq(42)
  end

  # Instance variables inside method definitions are OK
  def helper_method
    @internal_state = 42
    @other_var
  end

  def compute
    @result ||= expensive_call
  end
end
