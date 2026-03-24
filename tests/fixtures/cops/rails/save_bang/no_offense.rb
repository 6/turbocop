object.save!
object.update!
object.destroy('Tom')
Model.save(1, name: 'Tom')
object.save('Tom')
object.create!

# MODIFY method: return value assigned (exempt)
result = object.save
x = object.update(name: 'Tom')
@saved = object.save
@@flag = object.save
$global = object.save

# MODIFY method: return value in condition (exempt)
if object.save
  puts "saved"
end

# Parenthesized condition
if(object.save)
  puts "saved"
end

unless object.save
  puts "not saved"
end

object.save ? notify : rollback

# MODIFY method: return value in boolean expression (exempt)
object.save && notify_user
object.save || raise("failed")
object.save and log_success
object.save or raise("failed")

# Explicit return
def foo
  return object.save
end

# Implicit return (last expression in method, AllowImplicitReturn: true by default)
def bar
  object.save
end

# Implicit return in block body
items.each do |item|
  item.save
end

# Used as argument
handle_result(object.save)
log(object.update(name: 'Tom'))
handle_save(object.save, true)

# Used in array/hash literal that is assigned (return value used)
result = [object.save, object.update(name: 'Tom')]

# Hash value assigned
result = { success: object.save }

# Return with hash/array (argument context)
return [{ success: object.save }, true]

# Keyword argument
handle_save(success: object.save)

# Explicit early return from block (next)
objects.each do |object|
  next object.save if do_the_save
  do_something_else
end

# Explicit final return from block (next)
objects.each do |object|
  next foo.save
end

# CREATE method with persisted? check immediately
return unless object.create.persisted?

# ENV receiver is always exempt
ENV.update("DISABLE_SPRING" => "1")

# save/update with 2 arguments
Model.save(1, name: 'Tom')

# destroy with arguments is not a persistence method
object.destroy(param)

# CREATE with || in implicit return
def find_or_create(**opts)
  find(**opts) || create(**opts)
end

# CREATE with persisted? check on next line (local variable)
user = User.create
if user.persisted?
  foo
end


# CREATE with persisted? check on next line (instance variable)
@user = User.create
if @user.persisted?
  foo
end

# CREATE with persisted? check directly on result
return unless object.create.persisted?

# CREATE with persisted? in same if condition (parenthesized assignment)
if (user = User.create).persisted?
  foo(user)
end

# CREATE with persisted? after conditional assignment
user ||= User.create
if user.persisted?
  foo
end

# CREATE with persisted? checked non-immediately (skip intervening statements)
# RuboCop uses VariableForce to track all references in scope, not just next stmt.
def create_user
  user = User.create(user_params)
  logger.info("Attempting to create user")
  do_something_else
  if user.persisted?
    redirect_to user
  end
end

# CREATE with persisted? used in same expression (non-adjacent)
def create_and_render
  @user = User.create(user_params)
  render json: @user, status: @user.persisted? ? :created : :unprocessable_entity
end

# CREATE with persisted? in nested expression after other code
def process
  record = Record.find_or_create_by(name: params[:name])
  log_event("Processing record #{record.id}")
  raise ActiveRecord::RecordInvalid unless record.persisted?
end

# Persist call inside brace block — last expression (implicit return)
items.each { |i| i.save }

# Negation: ! / not on persist call (single_negative? in RuboCop — condition context)
!object.save
not object.save

# (Yield/super with persist call moved to offense.rb — RuboCop's
# argument? and implicit_return? don't treat yield/super as exempt)

# CREATE assigned to instance/class/global variable (RuboCop's VariableForce only tracks locals)
@record = object.first_or_create
@@record = User.create(name: 'Joe')
$record = User.create(name: 'Joe')

# CREATE assigned to instance variable without persisted? check (exempt — not tracked by VariableForce)
@user = User.create(name: 'Joe')
foo(@user)

# CREATE in ||= assignment (RuboCop's VariableForce doesn't flag or_asgn create-in-assignment)
label ||= Project.create(title: params[:title])

# CREATE in &&= assignment (same as ||=)
record &&= User.create(name: 'Joe')

# Persist call with block argument: create(hash, &block) has 2 args → not expected_signature
Model.create(name: 'Joe', &block)

# Setter receiver: persist call used as receiver of attribute write (assignment context)
# RuboCop treats setter methods (ending with =) as assignments.
def setter_examples
  create.multipart = true
  update.multipart = true
  save.flag = false
end

# Persist methods with literal arguments are not expected_signature (not AR persist calls)
# RuboCop's expected_signature? rejects literal args that aren't hashes.
create("string")
create("string_#{interpolation}")
create(:"sym_#{interpolation}")
create([{name: 'Joe'}, {name: 'Bob'}])
save([offense])
save(false)
create(true)
update([{id: 1, values: {name: 'Tom'}}])
create(42)
create(/regex/)
first_or_create([{name: 'parrot'}, {name: 'parakeet'}])

# CREATE inside array literal assigned to local variable (RuboCop does not flag)
# RuboCop's VariableForce check_assignment checks `if rhs_node.send_type?` — ArrayNode
# does not match, so create calls inside arrays in local assignments are skipped.
included = [
  Model.create(name: 'foo'),
  Model.create(name: 'bar')
]

matching = [
  Record.create(status: 'active'),
  Record.create(status: 'inactive')
]

# CREATE inside hash literal assigned to local variable (same reason)
lookup = {
  first: Model.create(name: 'first'),
  second: Model.create(name: 'second')
}

# MODIFY in || (return value checked as boolean — exempt)
x = record.save || raise("failed")
y = something || other

# MODIFY in || (return value checked as boolean — exempt)
# (yield and super with modify persist calls moved to offense.rb —
#  RuboCop's argument? doesn't treat yield/super as argument context)

# Block-bearing persist calls in Argument context are exempt.
# In RuboCop's Parser AST, `create { }` becomes Block(Send, Args, Body).
# assignable_node unwraps to block_node, then argument? checks block_node.parent.
# When the block is a direct argument to a method, argument? returns true (exempt).
@calc << InlineBox.create(width: 10, height: 20) {}
Glue.new(InlineBox.create(width: width, height: 10) {})
subscriptions.push(Subscription.create { queues.each {|q| q = [] }})
@buildings << Building.create { |b| b.build_owner(first_name: 'foo') }

# Hash#update and non-AR update with block as argument (exempt — argument context)
test_equal({"a"=>100, "b"=>200, "c"=>300}, h.update(h2) { |k, o, n| o })
expect(atomic.update { |v| v + 1 }).to eq 1001

# Create as keyword arg inside compound boolean — compound_boolean doesn't apply
# because the create's first ancestor is the method call (send), not the or/and node.
# RuboCop's in_condition_or_compound_boolean? checks the FIRST non-begin ancestor.
sash || reload.sash || update(sash: Sash.create)
x = clean_install || cache_version.version != Version.create(Metadata.cache_version)
Person.find_by(handle: id) || Person.new(key: key, pod: Pod.find_or_create_by(url: url))

# Assignment inside assert — persisted? checked on next statement
assert version = parent.versions.create(name: 'test', sharing: 'descendants')
assert version.persisted?

# CREATE in local assignment inside && in if predicate, with persisted? in if-body
# (RuboCop: return_value_assigned? catches lvasgn parent, VariableForce finds persisted?)
def create_in_and_predicate
  if (record = Creator.create(opts)) && record.present?
    increment if record.persisted?
  end
  nil
end

# Multi-write with create and block (return value assigned via masgn)
# RuboCop: assignable_node climbs block → parent is masgn → assignment? true
content, content_type = Formatter.create do |r|
  r.attach name: :data
end

# Multi-write with create inside block inside local variable assignment
# The outer `d3 = c.time_delta do ... end` sets in_local_assignment for d3,
# but the block body's multi-write should NOT inherit that flag.
d3 = c.time_delta do
  content, content_type = Taps::Multipart.create do |r|
    r.attach name: :encoded_data, payload: "data"
  end
end

# MODIFY method in operator-write assignment (+=, &=, etc.)
# RuboCop's return_value_assigned? checks assignable_node.parent.assignment?
# and op_asgn is in SHORTHAND_ASSIGNMENTS, so assignment? returns true.
success &= record.update(v: params[:v])
packet += @cipher.update(data)
total -= record.save

# MODIFY method in if-body structurally identical to if-condition (RuboCop value equality)
# RuboCop's in_condition_or_compound_boolean? uses == (not equal?) to compare nodes.
# When `save` appears both as the if-condition and in the if-body, RuboCop considers
# the body statement as being in condition context (exempted for modify methods).
def copy_from(project)
  Project.transaction do
    if save
      reload
      do_stuff
      save
    else
      false
    end
  end
end

# CREATE method with persisted? on different var in if-condition (RuboCop call_to_persisted? quirk)
# RuboCop's call_to_persisted? replaces node with if-condition when the reference is a
# parenthesized call in an if-body. So `user_export.update_columns(...)` inside
# `if upload.persisted?` triggers suppression even though persisted? is on `upload`, not `user_export`.
def execute
  user_export = UserExport.create(name: "x")
  user_export.id

  File.open("f") do |file|
    upload = UploadCreator.new(file).create_for(1)
    if upload.persisted?
      user_export.update_columns(upload_id: upload.id)
    end
  end
end

# Block-bearing create inside hash value inside collect_concat block (Tempfile.create is NOT AR)
# in_transparent_container should NOT leak through block boundaries
def fetch_articles(ftp, files)
  { articles: files.collect_concat do |file|
    Tempfile.create do |temp_file|
      ftp.getbinaryfile(file, temp_file.path)
      JSON.parse(File.read(temp_file.path))[:articles]
    end
  end }
end
