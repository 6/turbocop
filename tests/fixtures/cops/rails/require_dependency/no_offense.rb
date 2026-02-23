require 'some_lib'
require_relative 'concerns/authenticatable'
autoload :User, 'models/user'
include Authenticatable
extend ActiveSupport::Concern
load 'some_file.rb'
