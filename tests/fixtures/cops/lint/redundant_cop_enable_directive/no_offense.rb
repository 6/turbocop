# rubocop:disable Layout/LineLength
foooooooooooooooooooooooooooo = 1
# rubocop:enable Layout/LineLength
bar
# rubocop:disable Layout
x =   0
# rubocop:enable Layout
y = 2
# rubocop:disable Layout/LineLength
foooooo = 1
# rubocop:enable all

# Directives inside heredocs should not be detected
code = <<~RUBY
  foo = 1
  # rubocop:enable Layout/LineLength
  bar = 2
RUBY
puts code

# Trailing inline enables on non-comment-only lines are ignored
def inline_enable_help # rubocop:disable MethodLength
  <<~TEXT
    body
  TEXT
end # rubocop:enable MethodLength

# Doc comment containing embedded rubocop:disable counts as a real disable
# so the subsequent enable is valid (not redundant)
#   def f # rubocop:disable Style/For
#   end
# rubocop:enable Style/For

# Documentation text with embedded examples should not be treated as directives
# Checks that `# rubocop:enable ...` and `# rubocop:disable ...` statements
# are strictly formatted.

# Plain comment mentioning rubocop:enable in prose is not a directive
# removed. This is done in order to find rubocop:enable directives that
# have now become useless.

# Disable with trailing comment after cop name (not using -- separator)
# rubocop:disable Rails/FindEach # .each returns an array, .find_each returns nil
records.each do |record|
  process(record)
end
# rubocop:enable Rails/FindEach

# Enable with trailing punctuation on cop name
# rubocop:disable Metrics/MethodLength
def long_method
  x = 1
end
# rubocop:enable Metrics/MethodLength.

# Disable using the spaced directive form
# rubocop: disable Metrics/MethodLength
def medium_method
  value = 1
end
# rubocop:enable Metrics/MethodLength

# Nested disables require matching nested enables
# rubocop:disable Metrics/MethodLength
# rubocop:disable Metrics/MethodLength
def counted_method
  result = 1
end
# rubocop:enable Metrics/MethodLength
# rubocop:enable Metrics/MethodLength

# Nested examples in comments should not count as real directives
# rubocop:disable Lint/RedundantCopEnableDirective
# rubocop:disable Lint/RedundantCopDisableDirective
#   # rubocop:enable Layout/LineLength
#   # rubocop:disable Style/StringLiterals
# rubocop:enable Lint/RedundantCopDisableDirective
# rubocop:enable Lint/RedundantCopEnableDirective

# Trailing inline enable after a block disable is also ignored
# rubocop:disable Style/StringLiterals
foo('#')
bar('#') # rubocop:enable Style/StringLiterals

# Disable/enable for custom cop not in registry
# rubocop:disable Development/NoEvalCop
class_eval <<-RUBY
  def marshal_dump
    [@line, @col]
  end
RUBY
# rubocop:enable Development/NoEvalCop

# An outer specific disable remains active after an inner disable/enable all pair
module LlmClients
  module Openai
    #rubocop:disable Metrics/ClassLength
    class Client
      # rubocop:disable all
      def chat_request(chat_history, &)
        value = 1
        value
      end
      # rubocop:enable all

      def openai_connection
        @openai_connection ||= OpenAI::Client.new do |f|
          f.request :instrumentation, name: "req", instrumenter: self
        end
      end

      alias embed_connection openai_connection
      alias chat_connection openai_connection
    end
    #rubocop:enable Metrics/ClassLength
  end
end
