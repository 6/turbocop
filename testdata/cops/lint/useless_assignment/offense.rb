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
