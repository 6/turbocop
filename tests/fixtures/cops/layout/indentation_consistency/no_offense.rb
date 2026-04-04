require 'colorize'
require 'tmpdir'

def foo
  x = 1
  y = 2
  z = 3
end

class Bar
  a = 1
  b = 2
end

module Baz
  CONST = 1
  OTHER = 2
end

def single; end

if cond
  func1
  func2
end

if a1
  b1
elsif a2
  b2
else
  c
end

unless cond
  func1
  func2
end

case a
when b
  c
  c
when d
else
  f
end

while cond
  func1
  func2
end

until cond
  func1
  func2
end

for var in 1..10
  func1
  func2
end

begin
  func1
  func2
end

module VkontakteApi
  class Method
    def call(args = {}, &block)
      response = API.call(full_name, args, token)
      Result.process(response, type, block)
    end

  private
    def full_name
      parts = [@previous_resolver.name, @name].compact.map { |part| camelize(part) }
      parts.join(".").gsub(/[^A-Za-z.]/, "")
    end
  end
end

class A
  def _to_s(key)
    foo
  end; protected :_to_s

  def to_plain_s; _to_s(:a); end
end

def foo
  pnode =
    @node; loop do
      pnode = parent_node(pnode)
      break
    end
end

while a
end

for var in 1..10
end

if a
else
end

require 'ostruct'

module ClinicFinder
  module Modules
    module GestationHelper; end
  end
end

def erb(title:)
_erbout = +'';
_erbout.<< "<article>".freeze
; _erbout.<<(( ERB::Escape.html_escape(title) ).to_s);
_erbout.<< "</h3>".freeze
; _erbout.<<(( 'http://example.com' ).to_s);
_erbout.<< "</article>".freeze
; _erbout
end
