RSpec.describe 'test' do
  it 'flags block.call' do
    allow(foo).to receive(:bar) { |&block| block.call }
                                ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Yield: Use `.and_yield`.
  end

  it 'flags block.call with args' do
    allow(foo).to receive(:baz) { |&block| block.call(1, 2) }
                                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Yield: Use `.and_yield`.
  end

  it 'flags chained receive' do
    allow(foo).to receive(:qux).with(anything) { |&block| block.call }
                                               ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/Yield: Use `.and_yield`.
  end
end
