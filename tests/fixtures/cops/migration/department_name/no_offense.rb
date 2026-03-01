alias :ala :bala # rubocop:disable all
# rubocop:disable Style/Alias
alias :foo :bar
# rubocop:enable Style/Alias
# rubocop:disable Style/Alias -- because something, something, and something
alias :baz :qux
# rubocop:disable Style
alias :one :two
x = 1
y = 2

# plugin cops with nested department path are accepted
# rubocop:disable Discourse/Plugins/NamespaceConstants
# rubocop:enable Discourse/Plugins/NamespaceConstants

# multiple cop names separated by spaces (without commas) are accepted
# rubocop:disable RSpec/SubjectStub RSpec/MessageSpies
# rubocop:enable RSpec/SubjectStub RSpec/MessageSpies

# odd legacy-style token fragments containing slash are accepted
# rubocop:disable /BlockLength, Metrics/
