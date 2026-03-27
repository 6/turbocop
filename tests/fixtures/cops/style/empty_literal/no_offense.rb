a = []

h = {}

s = ''

a = Array.new(5)

h = Hash.new(0)

h = Hash.new { |h, k| h[k] = [] }

cache = Hash.new { Hash.new }

cache = Array.new { Array.new }

# String.new without frozen_string_literal comment should not be flagged
# (absence means potentially needing mutable strings)
s = String.new
