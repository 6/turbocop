#! /usr/bin/env ruby
# good comment
# another good comment
x = 1
y = 2
# frozen_string_literal: true

# RDoc toggle comments
#++
#--

# Multiple hashes
## section header

# Rackup options on first line of config.ru are allowed
# (tested via inline test below since fixture is not config.ru)

# Syntax-error files should not be checked by this cop.
#~# ORIGINAL retry

retry

#~# EXPECTED
retry
