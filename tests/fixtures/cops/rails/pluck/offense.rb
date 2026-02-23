users.map { |u| u[:name] }
^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Pluck: Use `pluck(:key)` instead of `map { |item| item[:key] }`.

posts.map { |p| p[:title] }
^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Pluck: Use `pluck(:key)` instead of `map { |item| item[:key] }`.

items.map { |item| item[:price] }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Pluck: Use `pluck(:key)` instead of `map { |item| item[:key] }`.

items.collect { |x| x[:key] }
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Pluck: Use `pluck(:key)` instead of `map { |item| item[:key] }`.

# Inside a receiverless block — nearest ancestor block has no receiver, so flag it
class_methods do
  built_in_agent_tools.map { |tool| tool[:id] }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Pluck: Use `pluck(:key)` instead of `map { |item| item[:key] }`.
end

do_something do
  items.map { |item| item[:name] }
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Pluck: Use `pluck(:key)` instead of `map { |item| item[:key] }`.
end
