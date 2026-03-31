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

# alias inside class_eval block should use alias_method (dynamic scope)
SomeClass.class_eval do
  alias new_name old_name
  ^^^^^ Style/Alias: Use `alias_method` instead of `alias`.
end

# alias inside module_eval block should use alias_method (dynamic scope)
SomeModule.module_eval do
  alias new_name old_name
  ^^^^^ Style/Alias: Use `alias_method` instead of `alias`.
end

alias :to_s :path
^ Style/Alias: Use `alias to_s path` instead of `alias :to_s :path`.

alias :clear :delete
^ Style/Alias: Use `alias clear delete` instead of `alias :clear :delete`.

alias :show :index
^ Style/Alias: Use `alias show index` instead of `alias :show :index`.

alias :reachable? :alive?
^ Style/Alias: Use `alias reachable? alive?` instead of `alias :reachable? :alive?`.

alias :method_missing :write
^ Style/Alias: Use `alias method_missing write` instead of `alias :method_missing :write`.

alias :inspect :to_s
^ Style/Alias: Use `alias inspect to_s` instead of `alias :inspect :to_s`.

alias :inspect :to_s
^ Style/Alias: Use `alias inspect to_s` instead of `alias :inspect :to_s`.

alias :browser_shutdown :shutdown
^ Style/Alias: Use `alias browser_shutdown shutdown` instead of `alias :browser_shutdown :shutdown`.

alias :[] :fetch
^ Style/Alias: Use `alias [] fetch` instead of `alias :[] :fetch`.
