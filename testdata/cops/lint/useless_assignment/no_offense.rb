def some_method
  some_var = 1
  do_something(some_var)
end

def other_method
  _unused = 1
  do_something
end

# Compound assignment += reads the variable, so the initial assignment is used
def compound_plus_equals
  count = 0
  3.times { count += 1 }
end

# Compound assignment in block
def compound_in_block
  rating = 1
  items.each { |item| item.update!(rating: rating += 1) }
end

# Or-assignment ||= reads the variable first
def or_assign
  hash_config = nil
  stub(:db_config, -> { hash_config ||= build_config }) { run }
end

# And-assignment &&= reads the variable first
def and_assign
  value = true
  value &&= check_condition
  do_something(value)
end

# Singleton method definition uses variable as receiver
def singleton_method_on_local
  conn = get_connection
  def conn.requires_reloading?
    true
  end
  pool.clear_reloadable_connections
end

# Another singleton method pattern
def define_method_on_object
  time = @twz.time
  def time.foo; "bar"; end
  @twz.foo
end

# Bare super implicitly forwards all method parameters
def self.instantiate_instance_of(klass, attributes, column_types = {}, &block)
  klass = superclass
  super
end

# String concatenation compound assignment
def compound_string_concat
  lines = "HEY\n" * 12
  assert_no_changes "lines" do
    lines += "HEY ALSO\n"
  end
end

# Variable assigned in block but read in nested block
describe "something" do
  it "does something" do
    app = create(:app)
    problem = create(:problem, app: app)
    expect do
      destroy(problem.id)
    end.to change(Problem, :count).by(-1)
  end
end

# Variable read inside same block (not nested)
items.each do |item|
  x = compute(item)
  process(x)
end

# Bare `binding` captures all local variables, so assignments are not useless
def render_template
  github_user = `git config github.user`.chomp
  template = File.read("template.erb")
  ERB.new(template).result(binding)
end

# `binding` in a block also captures all locals in that scope
task :announce do
  version = ENV["VERSION"]
  github_user = `git config github.user`.chomp
  puts ERB.new(template).result(binding)
end

# Variable assigned in block, read after block in outer scope (blocks share
# enclosing scope in Ruby for variables declared in the outer scope)
describe "block with outer read" do
  result = nil
  [1, 2, 3].each { |x| result = x * 2 }
  puts result
end

# Variable used across nested blocks (not siblings)
describe "nested blocks" do
  it "works" do
    token = create(:token)
    3.times do
      validate(token)
    end
  end
end

# All sibling blocks use their own token (each is used)
describe "all siblings used" do
  it "first" do
    token = create(:token)
    expect(token).to be_valid
  end
  it "second" do
    token = create(:token)
    expect(token).to be_present
  end
end

# `binding` in a nested block captures locals from the outer block scope
describe "binding in nested block" do
  version = "1.0"
  channel = "stable"
  items.each { puts ERB.new(tmpl).result(binding) }
end

# Variable assigned in block and read in sibling block's descendant (via
# ancestor scope) — this is NOT a sibling read, the outer describe scope
# sees the read.
describe "ancestor read" do
  total = 0
  items.each { |x| total += x }
  it "checks total" do
    expect(total).to eq(42)
  end
end

# Variable initialized to nil, reassigned inside a lambda, read after block.
# Common in Rails test stubs — the lambda captures the outer variable.
describe "lambda capture reassignment" do
  it "captures display image" do
    display_image_actual = nil
    stub :show, ->(img) { display_image_actual = img } do
      take_screenshot
    end
    assert_match(/screenshot/, display_image_actual)
  end
end

# Multiple variables captured by lambdas at different nesting levels
describe "multi-level lambda capture" do
  it "captures at different levels" do
    captured_a = nil
    captured_b = false
    stub :foo, ->(x) { captured_a = x } do
      stub :bar, -> { captured_b = true } do
        run_action
      end
    end
    assert captured_b
    assert_match(/expected/, captured_a)
  end
end

# RSpec `.change { var }` matcher — the block reads the variable
describe "change matcher reads variable" do
  it "tracks changes" do
    count = 0
    items.each { count += 1 }
    expect { do_something }.to change { count }
  end
end

# Variable assigned in parent block, written+read across multiple siblings
# (the "error = nil" Rails pattern)
describe "shared variable across siblings" do
  error = nil
  it "assigns error" do
    error = validate(input)
  end
  it "checks error" do
    assert_nil error
  end
end

# Accumulator pattern — array initialized in parent scope, appended in block,
# read in sibling block (common in Rails test setup)
describe "accumulator across siblings" do
  sponsors = []
  users.each { |u| sponsors << u if u.sponsor? }
  it "has sponsors" do
    expect(sponsors).not_to be_empty
  end
end

# Three-level nesting: describe > context > it, variable in describe read in it
describe "deep nesting" do
  shared_val = compute_value
  context "when enabled" do
    it "uses shared_val" do
      expect(shared_val).to eq(42)
    end
  end
end
