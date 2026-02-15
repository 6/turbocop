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
end
