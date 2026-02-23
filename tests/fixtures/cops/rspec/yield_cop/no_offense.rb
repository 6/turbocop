RSpec.describe 'test' do
  it 'allows receive with no block args' do
    allow(foo).to receive(:bar) { |block| block.call }
  end

  it 'allows block.call with extra statements' do
    allow(foo).to receive(:bar) do |&block|
      result = block.call
      transform(result)
    end
  end

  it 'uses and_yield' do
    allow(foo).to receive(:bar).and_yield
  end
end
