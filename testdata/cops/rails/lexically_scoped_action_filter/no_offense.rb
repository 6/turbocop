class UsersController < ApplicationController
  before_action :authenticate, only: [:index]

  def index
  end
end

class Auth::PasswordsController < Devise::PasswordsController
  before_action :redirect, only: :edit, unless: :token_valid?

  def update
    super
  end
end
