str.match?(/foo/)
CONST = str.match?(/#{ANOTHER_CONST}/)
@regexp ||= /#{ANOTHER_CONST}/
str.match?(/#{CONST}/o)
str.match?(/#{CONST}#{do_something(1)}/)
