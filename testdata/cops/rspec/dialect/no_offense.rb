describe 'display name presence' do
  it 'tests common context invocations' do
    expect(request.context).to be_empty
  end
end

RSpec.describe 'top level' do
  it 'works' do
    expect(true).to eq(true)
  end
end
