# -*- encoding : utf-8 -*-
class ApplicationController < ActionController::Base
^ Style/Documentation: Missing top-level documentation comment for `class`.
  protect_from_forgery with: :exception
end

class Foo
^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  def method
  end
end

module Bar
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
  def method
  end
end

class MyClass
^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  def method
  end
end

module MyModule
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
  def method
  end
end

module Test
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
end

module MixedConcern
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
  extend ActiveSupport::Concern

  module ClassMethods
  ^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
    def some_method
    end
  end
end

module Types
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
  include Dry::Types()
end

class Base
^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  include ActionDispatch::Routing::RouteSet.new.url_helpers
end

unless Object.const_defined?(:AccordionSection2)
  # Note: this is similar to AccordionSection in HelloComponentSlots but specifies default_slot for simpler consumption
  class AccordionSection2
  ^ Style/Documentation: Missing top-level documentation comment for `class`.
    class Presenter
    end

    attr_reader :collapsed
  end
end

# Note: named Address2 to avoid conflicting with other samples if loaded together
class Address2
^ Style/Documentation: Missing top-level documentation comment for `class`.
  attr_accessor :text
end

#!/usr/bin/env ruby
class SnippetsExample
^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  def say_hello(name)
    puts "Hello, #{name}"
  end
end

#!/bin/env ruby
# encoding: utf-8
class CreateWkAccounting < ActiveRecord::Migration[4.2]
^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  def change
  end
end

#coding : utf-8
module NoticesHelper
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
  def mobile?(call_number)
    call_number.present? and call_number.size == 11
  end
end

class Foo
^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  class << self
  end
end

# outer docs
module Foo; class Bar
            ^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  def method
  end
end; end

# real doc
module UserVars
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
  class << self
    attr_accessor :autostart_scripts
  end

  self.autostart_scripts = []
end unless defined?(UserVars)

begin
  # comment
  class Tester
  ^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
    def method
    end
  end
rescue LoadError
end

class ::Object #:nodoc:
^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  def meta_class
    class << self; self end
  end
end

class FormatParser::DPXParser
^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  class Inner; end
  def method; end
end

# frozen_string_literal: true

module Iguvium
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
  GAUSS = 1
  def method; end
end

layout = class TestStruct < FFI::Struct
         ^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  layout :i, :int
end

out = class Cor
      ^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  def blimey; end
end

class FormatParser::DPXParser
^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
  # Helper docs
  class Binstr
    def parse
    end
  end

  private_constant :Binstr, :Capture
end

module Iguvium
^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
  GAUSS = 1
  HORIZONTAL = 2

  # Performs all the work
  class CV
    def recognize
    end
  end

  private_constant :GAUSS, :HORIZONTAL
end

module Backports
  class Ractor
  ^^^^^ Style/Documentation: Missing top-level documentation comment for `class`.
    # Base queue docs
    class BaseQueue
      def pop_non_blocking
      end
    end

    # Incoming queue docs
    class IncomingQueue < BaseQueue
      def reenter
      end
    end

    private_constant :BaseQueue, :IncomingQueue
  end
end

require 'fiber'

# outer docs
module RailsERD
  module Inspectable # @private :nodoc:
  ^^^^^^ Style/Documentation: Missing top-level documentation comment for `module`.
    def inspection_attributes(*attributes)
    end
  end
end
