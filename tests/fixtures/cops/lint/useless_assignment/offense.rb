def some_method
  some_var = 1
  ^^^^^^^^ Lint/UselessAssignment: Useless assignment to variable - `some_var`.
  do_something
end

def other_method
  x = compute_value
  ^ Lint/UselessAssignment: Useless assignment to variable - `x`.
  y = another_value
  do_something(y)
end

def third_method
  unused = 'hello'
  ^^^^^^ Lint/UselessAssignment: Useless assignment to variable - `unused`.
end

# Useless assignment inside a block (not inside a def)
describe "something" do
  it "does something" do
    problem = create(:problem)
    ^^^^^^^ Lint/UselessAssignment: Useless assignment to variable - `problem`.
    expect(true).to eq(true)
  end
end

# Useless assignment in sibling block — each `it` block is an independent
# closure. A variable written in one sibling is NOT accessible in another.
describe "matching tokens" do
  it "uses token" do
    token = FactoryBot.create(:access_token)
    expect(last_token).to eq(token)
  end
  it "does not use token" do
    token = FactoryBot.create(:access_token)
    ^^^^^ Lint/UselessAssignment: Useless assignment to variable - `token`.
    last_token = described_class.matching_token_for(application)
    expect(last_token).to eq(nil)
  end
end

# Useless in one sibling, used in another (only the unused one is flagged)
RSpec.describe "examples" do
  context "first" do
    result = compute_something
    ^^^^^^ Lint/UselessAssignment: Useless assignment to variable - `result`.
    expect(true).to be(true)
  end
  context "second" do
    result = compute_something
    use(result)
  end
end

# Useless assignment inside a lambda block
describe "lambda with unused var" do
  it "does not use val" do
    callback = ->(x) {
      val = x * 2
      ^^^ Lint/UselessAssignment: Useless assignment to variable - `val`.
      puts "done"
    }
    callback.call(5)
  end
end

# Deeply nested sibling blocks — each `it` is still independent
describe "outer" do
  context "inner" do
    it "first" do
      data = fetch_data
      ^^^^ Lint/UselessAssignment: Useless assignment to variable - `data`.
      expect(true).to eq(true)
    end
    it "second" do
      data = fetch_data
      use(data)
    end
  end
end
