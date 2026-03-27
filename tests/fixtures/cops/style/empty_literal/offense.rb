# frozen_string_literal: false
a = Array.new
    ^^^^^^^^^ Style/EmptyLiteral: Use array literal `[]` instead of `Array.new`.

h = Hash.new
    ^^^^^^^^ Style/EmptyLiteral: Use hash literal `{}` instead of `Hash.new`.

s = String.new
    ^^^^^^^^^^ Style/EmptyLiteral: Use string literal `''` instead of `String.new`.

values = Hash.new { |hash, key| hash[key] = Hash.new }
                                            ^^^^^^^^ Style/EmptyLiteral: Use hash literal `{}` instead of `Hash.new`.

@token_regexps       = Hash.new { |h,k| h[ k ] = Hash.new }
                                                 ^^^^^^^^ Style/EmptyLiteral: Use hash literal `{}` instead of `Hash.new`.

queues = Array.new(n) {|i| Array.new }
                           ^^^^^^^^^ Style/EmptyLiteral: Use array literal `[]` instead of `Array.new`.
