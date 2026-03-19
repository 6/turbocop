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

# Yield with persist call argument (return value used by yield)
def process
  yield object.save
end

# Yield with persist call as non-last statement (still an argument)
def process
  yield object.save
  nil
end

# Super with persist call argument
def process
  super(object.save)
  nil
end

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

# Operator-write assignment (+=, -=, etc.) — return value is used
def process
  packet += cipher.update(data)
  nil
end

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
