{foo: 1, bar: 2, baz: 3}.reject { |k, v| k == :bar }
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(:bar)` instead.
{foo: 1, bar: 2, baz: 3}.select { |k, v| k != :bar }
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(:bar)` instead.
{foo: 1, bar: 2, baz: 3}.filter { |k, v| k != :bar }
                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(:bar)` instead.
hash.reject { |k, v| k == 'str' }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except('str')` instead.
hash.reject { |k, _| [:foo, :bar].include?(k) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(:foo, :bar)` instead.
hash.reject { |k, _| KEYS.include?(k) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(*KEYS)` instead.
hash.select { |k, _| !KEYS.include?(k) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(*KEYS)` instead.
hash.filter { |k, _| !KEYS.include?(k) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(*KEYS)` instead.
hash.reject { |k, _| excluded.include?(k) }
     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(*excluded)` instead.
hash.reject do |k, _|
     ^^^^^^^^^^^^^^^^^ Style/HashExcept: Use `except(*excluded)` instead.
  excluded.include?(k)
end

create_parameters = parameters.reject{ |k, _v| k.eql?(:grant_method)}
                               ^ Style/HashExcept: Use `except(:grant_method)` instead.

oauth_parameters = @auth_params.reject{ |k, _v| k.eql?(:type)}
                                ^ Style/HashExcept: Use `except(:type)` instead.

r.table_create(table_name, create_options.reject { |k,_| k.in? [:name, :write_acks] })
                                          ^ Style/HashExcept: Use `except(:name, :write_acks)` instead.

def except(*keys)                 = reject { |key, _| keys.include? key }
                                    ^ Style/HashExcept: Use `except(*keys)` instead.

reject {|k,v| args.include?(k) }
^ Style/HashExcept: Use `except(*args)` instead.

.reject { |code, _name| code.in?(EXCLUDED_COUNTRY_CODES) }
 ^ Style/HashExcept: Use `except(*EXCLUDED_COUNTRY_CODES)` instead.

Rails::Command.printing_commands.reject do |command, _|
                                 ^ Style/HashExcept: Use `except(*COMMANDS_IN_USAGE)` instead.
  command.in?(COMMANDS_IN_USAGE)
end

reject do |key, _|
^ Style/HashExcept: Use `except(*keys)` instead.
  keys.include?(key)
end
