require 'json'
require 'net/http'
require 'fileutils'
require 'yaml'
require 'csv'
# 'set' is only redundant in Ruby 3.2+; at default 2.7, it's NOT redundant
require 'set'
# 'fiber' is only redundant in Ruby 3.1+; at default 2.7, it's NOT redundant
require 'fiber'
