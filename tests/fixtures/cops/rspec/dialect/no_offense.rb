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

it 'raises an error' do
  expect { subject }.to raise_error(StandardError)
end

expect { run }.to raise_error(ArgumentError, 'bad input')

# Non-RSpec method named context on a receiver is fine
request.context
user.raise_exception
