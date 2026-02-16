FILES_TO_LINT = Dir['lib/*.rb']

class TestRequest; end

LIST = []

class TestEnum < T::Enum
  enums do
    Foo = new("foo")
  end
end

module M
  CONSTANT = 1
end

# Namespaced constant writes in blocks are intentional (explicit scope)
config.before_configuration do
  ::REDIS_CONFIGURATION = RedisConfiguration.new
end

task :setup do
  Module::SETTING = true
end
