object.save!
object.update!
object.destroy('Tom')
Model.save(1, name: 'Tom')
object.save('Tom')
object.create!

# Return value assigned
result = object.save
x = object.update(name: 'Tom')
@saved = object.save
@@flag = object.save
$global = object.save

# Return value in condition
if object.save
  puts "saved"
end

unless object.save
  puts "not saved"
end

object.save ? notify : rollback

# Return value in boolean expression
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

# Used in array/hash literal
[object.save, object.update(name: 'Tom')]
