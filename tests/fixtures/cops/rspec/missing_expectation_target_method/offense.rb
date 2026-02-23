RSpec.describe 'test' do
  it 'something' do
    something = 1
    expect(something).kind_of? Integer
                      ^^^^^^^^ RSpec/MissingExpectationTargetMethod: Use `.to`, `.not_to` or `.to_not` to set an expectation.
  end

  it 'uses eq? directly' do
    expect(something).eq? 42
                      ^^^ RSpec/MissingExpectationTargetMethod: Use `.to`, `.not_to` or `.to_not` to set an expectation.
  end

  it 'uses == with is_expected' do
    is_expected == 42
                ^^ RSpec/MissingExpectationTargetMethod: Use `.to`, `.not_to` or `.to_not` to set an expectation.
  end
end
