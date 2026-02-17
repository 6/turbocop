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
