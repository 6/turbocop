RSpec.describe Foo do
  it 'does too much' do
  ^^^^^^^^^^^^^^^^^^^^^ RSpec/ExampleLength: Example has too many lines. [6/5]
    line_1
    line_2
    line_3
    line_4
    line_5
    line_6
  end

  specify 'also too long' do
  ^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ExampleLength: Example has too many lines. [7/5]
    a = 1
    b = 2
    c = 3
    d = 4
    e = 5
    f = 6
    g = 7
  end

  it 'just barely over' do
  ^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ExampleLength: Example has too many lines. [6/5]
    step_one
    step_two
    step_three
    step_four
    step_five
    step_six
  end
end

it "handles block comment followed by line comment" do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ExampleLength: Example has too many lines. [7/5]
  ss = stmts <<-eof
# comments1

=begin
comments2
=end
# comments3
def hello; end
eof
  expect(ss.last.comments).to eq "comments3"
end

it "handles block comment followed by block comment" do
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ RSpec/ExampleLength: Example has too many lines. [10/5]
  ss = stmts <<-eof
=begin
comments1
=end
=begin
comments2
=end
def hello; end
eof
  expect(ss.last.comments).to eq "comments2"
end
