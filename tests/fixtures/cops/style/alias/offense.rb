alias_method :bar, :foo
^^^^^^^^^^^^ Style/Alias: Use `alias` instead of `alias_method`.

alias_method :new_name, :old_name
^^^^^^^^^^^^ Style/Alias: Use `alias` instead of `alias_method`.

alias_method :greet, :hello
^^^^^^^^^^^^ Style/Alias: Use `alias` instead of `alias_method`.

class C
  alias_method :ala, :bala
  ^^^^^^^^^^^^ Style/Alias: Use `alias` instead of `alias_method`.
end

module M
  alias_method :ala, :bala
  ^^^^^^^^^^^^ Style/Alias: Use `alias` instead of `alias_method`.
end

SomeClass.class_eval do
  alias_method :ala, :bala
  ^^^^^^^^^^^^ Style/Alias: Use `alias` instead of `alias_method`.
end

SomeModule.module_eval do
  alias_method :ala, :bala
  ^^^^^^^^^^^^ Style/Alias: Use `alias` instead of `alias_method`.
end
