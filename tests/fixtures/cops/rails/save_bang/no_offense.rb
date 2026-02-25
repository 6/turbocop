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

# Used in array/hash literal
[object.save, object.update(name: 'Tom')]

# Hash value
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
