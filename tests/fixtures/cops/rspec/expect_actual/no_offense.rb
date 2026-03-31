describe Foo do
  it 'uses expect correctly' do
    expect(foo).to eq(bar)
    expect(some_method).to eq(123)
    expect(result).to be_truthy
    expect(object.name).to eq("expected")
  end

  # Literal with no-arg matcher is not flagged (e.g. Capybara be_present)
  it 'allows literal with argumentless matcher' do
    expect(".css-selector").to be_present
    expect("path").to be_routable
  end

  # route_to matcher is always skipped
  it 'allows route_to' do
    expect("/users/1").to route_to(controller: "users", action: "show")
  end

  # Matcher chaining (`with`, `in_file`) is outside the matched runner shape
  it 'allows chained matcher receivers' do
    expect(:event_name).to have_been_published.with(payload: value)
  end

  # Runner calls with explicit failure messages are not matched
  it 'allows to with an explicit failure message' do
    expect([200, 204]).to include(status), "unexpected status #{status}"
  end

  # Multiline string literals parse as dynamic strings in RuboCop's AST
  it 'allows multiline string expect actual values' do
    expect('
      module Demo
        def value = 1
      end
    ').to eq(actual_code)
  end

  # Matcher blocks bind to the runner path and RuboCop does not match them
  it 'allows matcher calls with attached blocks' do
    expect(5).to arbitrary_matcher { 5 }
    expect(5).to arbitrary_matcher(expected: 4) { 5 }
    expect(5).to arbitrary_matcher(expected: 4).with(5) { 3 }
    expect(true).to satisfy("be true") { |val| val }
    expect(false).not_to satisfy("be true") { |val| val }
  end

  it 'allows matcher blocks inside block expectations' do
    expect {
      expect(false).to satisfy("be true") { |val| val }
    }.to fail_with("expected false to be true")

    expect {
      expect(true).not_to satisfy("be true") { |val| val }
    }.to fail_with("expected true not to be true")
  end
end
