CONST = [1, 2, 3].freeze

CONST2 = { a: 1, b: 2 }.freeze

CONST3 = 'str'.freeze

CONST4 = "top#{1 + 2}".freeze

TOP_TEST = Something.new.freeze

# to_i, to_f, to_r, to_c produce immutable values but vendor does not flag them
TIMEOUT = ENV['TIMEOUT'].to_i.freeze
RATE = ENV['RATE'].to_f.freeze
RATIO = ENV['RATIO'].to_r.freeze
COMPLEX = ENV['COMPLEX'].to_c.freeze

# String concatenation produces mutable strings, freeze is meaningful
COMBINED = (PART_A + PART_B + PART_C).freeze

# Constant + constant concatenation
MARKUP = (CLOSE_TAG + OPEN_TAG).freeze
