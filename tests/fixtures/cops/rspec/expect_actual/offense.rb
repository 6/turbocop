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
    expect(__FILE__).to eq(expected_path)
           ^^^^^^^^ RSpec/ExpectActual: Provide the actual value you are testing to `expect(...)`.
    expect(true).to satisfy("be true") do |val|
           ^^^^ RSpec/ExpectActual: Provide the actual value you are testing to `expect(...)`.
      val
    end
    expect(false).not_to satisfy("be true") do |val|
           ^^^^^ RSpec/ExpectActual: Provide the actual value you are testing to `expect(...)`.
      val
    end
  end
end
