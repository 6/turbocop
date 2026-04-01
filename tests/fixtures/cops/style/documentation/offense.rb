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
