class UsersController < ApplicationController
  before_action :authenticate, only: [:index]

  def index
  end
end
