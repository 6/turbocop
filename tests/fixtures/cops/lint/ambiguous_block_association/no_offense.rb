some_method(a) { |el| puts el }
some_method(a) do; puts a; end
some_method a do; puts "dev"; end
Foo.bar(a) { |el| puts el }
foo == bar { baz a }
foo ->(a) { bar a }
scope :active, -> { where(status: "active") }
# lambda/proc/Proc.new are block builders, never ambiguous
scope :active, lambda { where(status: "active") }
scope :active, proc { where(status: "active") }
scope :active, Proc.new { where(status: "active") }
foo = lambda do |diagnostic|; end
# Inner call with arguments (parens) — block clearly belongs to inner call
env ENV.fetch("ENV") { "dev" }
config.pam_default_suffix = ENV.fetch('PAM_EMAIL_DOMAIN') { ENV['LOCAL_DOMAIN'] }
environment ENV.fetch('RAILS_ENV') { 'development' }
self.cached_tallies = options.map { 0 }
also_known_as = as_array(json).map { |item| value_or_id(item) }
