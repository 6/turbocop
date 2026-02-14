describe Foo do
  it 'uses expect incorrectly' do
    expect(123).to eq(bar)
           ^^^ RSpec/ExpectActual: Provide the actual value you are testing to `expect(...)`.
    expect(true).to eq(bar)
           ^^^^ RSpec/ExpectActual: Provide the actual value you are testing to `expect(...)`.
    expect("foo").to eq(bar)
           ^^^^^ RSpec/ExpectActual: Provide the actual value you are testing to `expect(...)`.
    expect(nil).to eq(bar)
           ^^^ RSpec/ExpectActual: Provide the actual value you are testing to `expect(...)`.
    expect(:sym).to eq(bar)
           ^^^^ RSpec/ExpectActual: Provide the actual value you are testing to `expect(...)`.
  end
end
