# TODO: make better

# FIXME: does not work

# OPTIMIZE: this loop

# HACK: temporary workaround

# REVIEW: this logic

# NOTE: important detail

# Multi-line comment blocks: only first line is checked
# Note: this is a continuation line, should not be flagged
# TODO this is also a continuation line

# Schema annotation with extra spaces before keyword
#  note                          :text             default(""), not null

# Capitalized keyword as a word in a sentence (just_keyword_of_sentence)
# Note that this is important
# Fixme later if needed
# Hack the planet is a movie reference
# Optimize your workflow by using shortcuts
# Review the documentation for details

# Inline bare keyword — not an annotation per RuboCop
require 'something' if defined?(::Foo) # HACK

# Bare keyword alone (no colon or space after) — not an annotation per RuboCop

# TODO

# FIXME

# HACK

# NOTE

# REVIEW

# OPTIMIZE

# Todo

# Fixme
