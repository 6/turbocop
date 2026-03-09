# Copyright 2025 Acme Inc.

class Utils
  attr_reader :name,
              :status,
              :role,

  # ArrayLiteralInRegexp (must use literal array inside interpolation, not a variable)
  def match_keywords(text)
    text.match?(/#{%w[error warning fatal]}/)
  end

  def match_codes(text)
    text.match?(/#{["E001", "E002", "E003"]}/)
  end

  def match_ids(text)
    text.match?(/#{[1, 2, 3]}/)
  end

  # DuplicateRescueException
  def safe_parse(input)
    JSON.parse(input)
  rescue JSON::ParserError
    nil
  rescue JSON::ParserError
    {}
  end

  def safe_convert(input)
    Integer(input)
  rescue ArgumentError
    nil
  rescue ArgumentError
    0
  end

  # PercentSymbolArray (colons inside %i are what the cop detects)
  def symbol_list
    %i(:foo :bar :baz)
  end

  # RegexpAsCondition
  def check_pattern(line)
    if /error/
      puts "matched"
    end

    if /warning/
      puts "also matched"
    end

    if /fatal/i
      puts "critical"
    end
  end
end

# RedundantConstantBase (at top level, :: prefix is redundant)
TOP_TIME = ::Time.now
TOP_DATE = ::Date.today
TOP_HOME = ::ENV["HOME"]

# DoubleCopDisableDirective
x = 1 # rubocop:disable Style/Foo # rubocop:disable Style/Bar
y = 2 # rubocop:disable Lint/Baz # rubocop:disable Lint/Qux
z = 3 # rubocop:disable Style/A # rubocop:disable Style/B
