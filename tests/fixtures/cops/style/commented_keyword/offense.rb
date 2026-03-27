if x
  y
end # comment
    ^^^^^^^^^ Style/CommentedKeyword: Do not place comments on the same line as the `end` keyword.

begin # comment
      ^^^^^^^^^ Style/CommentedKeyword: Do not place comments on the same line as the `begin` keyword.
  y
end

class X # comment
        ^^^^^^^^^ Style/CommentedKeyword: Do not place comments on the same line as the `class` keyword.
  y
end

module X # comment
         ^^^^^^^^^ Style/CommentedKeyword: Do not place comments on the same line as the `module` keyword.
  y
end

def x # comment
      ^^^^^^^^^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.
  y
end

def x(a, b) # comment
            ^^^^^^^^^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.
  y
end

def self.append_log dir, txt#, prefix=''
                            ^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.

class IpGeocodeLookupTest < ActionDispatch::IntegrationTest#TestCase
                                                           ^ Style/CommentedKeyword: Do not place comments on the same line as the `class` keyword.

def self.pathify_actions result, structure#, name
                                          ^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.

class Foo#comment
         ^ Style/CommentedKeyword: Do not place comments on the same line as the `class` keyword.

def bar#comment
       ^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.

module Baz#comment
          ^ Style/CommentedKeyword: Do not place comments on the same line as the `module` keyword.

def black_king_move_up;   piece_move_o("59", "58", "☗5八玉"); end # 1手目
                                                                ^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.

def white_king_move_up;   piece_move_o("51", "52", "☖5二玉"); end # 2手目
                                                                ^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.

def black_king_move_down; piece_move_o("58", "59", "☗5八玉"); end # 3手目
                                                                ^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.

def white_king_move_down; piece_move_o("52", "51", "☖5一玉"); end # 4手目
                                                                ^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.

module Inspectable # @private :nodoc:
                   ^ Style/CommentedKeyword: Do not place comments on the same line as the `module` keyword.

def output # @private :nodoc:
           ^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.

def foreign_key_method_name # @private :nodoc:
                            ^ Style/CommentedKeyword: Do not place comments on the same line as the `def` keyword.
