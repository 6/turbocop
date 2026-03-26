  # encoding: utf-8
  require "logstash/filters/base"

def foo() end

# encoding: us-ascii
def bar() end

# frozen_string_literal: true
def baz() end

x = 1
y = 2
z = x + y
