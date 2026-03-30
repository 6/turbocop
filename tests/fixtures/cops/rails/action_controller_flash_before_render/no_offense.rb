class UsersController < ApplicationController
  def create
    flash.now[:notice] = "Created"
    render :new
  end
end

class PostsController < ApplicationController
  def create
    flash[:notice] = "Created"
    redirect_to posts_path
  end
end

# Non-controller class should not trigger
class NonController < ApplicationRecord
  def create
    flash[:alert] = "msg"
    render :index
  end
end

# flash before redirect_back should not trigger
class HomeController < ApplicationController
  def create
    if condition
      flash[:alert] = "msg"
    end
    redirect_back fallback_location: root_path
  end
end

# flash in if block with redirect_to at outer level
class RecordsController < ApplicationController
  def create
    if condition
      do_something
      flash[:alert] = "msg"
    end
    redirect_to :index
  end
end

# flash before redirect_to with return inside if
class SessionsController < ApplicationController
  def create
    if condition
      flash[:alert] = "msg"
      redirect_to :index
      return
    end
    render :index
  end
end

# flash inside iteration block before redirect_to
class MessagesController < ApplicationController
  def create
    messages = %w[foo bar baz]
    messages.each { |message| flash[:alert] = message }
    redirect_to :index
  end
end

# class method should not trigger
class HomeController < ApplicationController
  def self.create
    flash[:alert] = "msg"
    render :index
  end
end

# Qualified superclass: RuboCop does NOT match Admin::ApplicationController
class ImportController < Admin::ApplicationController
  def create
    flash[:alert] = "Import failed"
    render :new
  end
end

# Deeply qualified superclass: RuboCop does NOT match
class PostsController < Decidim::Admin::ApplicationController
  def create
    flash[:alert] = "msg"
    render :edit
  end
end

# Flash inside if branch with no outer siblings — NOT implicit render
# RuboCop only applies implicit render at def level, not inside branches
class RestroomsController < ApplicationController
  def display_errors
    if @restroom.errors.any?
      flash[:alert] = "unexpected"
    end
  end
end

# Flash + render inside rescue body — RuboCop walks up to rescue ancestor,
# finds no render in outer siblings, so no offense
class CoursesController < ApplicationController
  def create_from_tar
    begin
      do_something
    rescue StandardError => e
      flash[:error] = "Error -- #{e.message}"
      render(action: "new") && return
    end
  end
end

# Flash + render inside if branch — RuboCop checks outer siblings of if, not inner
class TasksController < ApplicationController
  def update
    if @task.save
      redirect_to @task
    else
      flash[:error] = "Save failed"
      render :edit
    end
  end
end

# Flash in rescue with render && return — rescue ancestor has no right siblings
# Method body after begin/end has more code but RuboCop doesn't check it
class ArchivesController < ApplicationController
  def create_from_tar
    begin
      do_something
    rescue SyntaxError => e
      flash[:error] = "Parse error"
      render(action: "new") && return
    rescue StandardError => e
      flash[:error] = "Read error"
      render(action: "new") && return
    end

    begin
      save_result
    rescue StandardError => e
      flash[:error] = "Extract error"
      render(action: "new") && return
    end

    unless @record.save
      flash[:error] = "Save failed"
      render(action: "new") && return
    end

    redirect_to @record
  end
end

# respond_to with flash in format.html (with custom redirect) and render in sibling format.api
# RuboCop walks up to the if ancestor; its right_siblings are empty, so no offense.
class CategoriesController < ApplicationController
  def create
    if @category.save
      respond_to do |format|
        format.html do
          flash[:notice] = "Created"
          redirect_to_settings_in_projects
        end
        format.api do
          render action: 'show', status: :created
        end
      end
    end
  end
end

# respond_to with flash in format.html, format.js (no block), format.api with render
class VersionsController < ApplicationController
  def create
    if @version.save
      respond_to do |format|
        format.html do
          flash[:notice] = "Created"
          redirect_to_settings_in_projects
        end
        format.js
        format.api do
          render action: 'show', status: :created
        end
      end
    end
  end
end

# Flash in else branch of if/elsif/else — RuboCop checks first if ancestor's
# right siblings which are empty (elsif is nested inside the outer if)
class AccountsController < ApplicationController
  def create
    if admin?
      @user = User.new(admin_params)
    elsif moderator?
      @user = User.new(mod_params)
    else
      flash[:error] = "Permission denied"
      redirect_to(root_path) && return
    end

    if @user.save
      redirect_to @user
    else
      render action: "new"
    end
  end
end

# Flash in deeply nested if inside another if — only innermost if ancestor
# is checked, and its right siblings within parent are empty
class ProfilesController < ApplicationController
  def destroy
    if authorized_to_delete?
      if @record.present?
        if @record.active?
          flash[:notice] = "Deactivated"
        else
          flash[:error] = "Already inactive"
        end
      end
    end

    respond_to do |format|
      format.html { render "show" }
    end
  end
end

# Flash in rescue body with redirect inside nested begin/rescue — rescue ancestor
# has no right siblings, method body after has render but RuboCop doesn't check
class SetupController < ApplicationController
  def install
    begin
      build_folder
    rescue StandardError => e
      flash[:error] = e.to_s
      begin
        cleanup_folder
      rescue StandardError => e2
        flash[:error] += "Recovery error: #{e2}"
        redirect_to(action: :retry)
        return
      end
    end

    flash[:success] = "Installed"
    redirect_to root_path
  end
end

# Flash in single-statement block before redirect — outer redirect visible
class AlertsController < ApplicationController
  def create
    messages.each { |message| flash[:alert] = message }
    redirect_to :index
  end
end

# Flash in main body of def-with-rescue: rescue ancestor suppresses implicit render
# (RuboCop checks rescue's right_siblings, which are empty — no render found)
class TokensController < ApplicationController
  def revoke
    token.destroy
    flash[:info] = "Token revoked"
  rescue StandardError => e
    flash[:error] = e.message
  ensure
    redirect_to action: :index
  end
end

# Flash in main body of def-with-rescue (no ensure): rescue ancestor's right_siblings
# are empty, so no render is found
class CurrencyController < ApplicationController
  def update_currency
    record.update!(value: new_value)
    flash[:success] = "Updated"
  rescue ActiveRecord::RecordInvalid => e
    flash[:error] = e.message
  end
end

# Flash in rescue body with redirect_to in ensure — no render in ensure
class RssController < ApplicationController
  def create
  rescue StandardError => e
    flash[:alert] = "Error"
  ensure
    redirect_to :index
  end
end

# Subtree-matched class with nested controller inside a module:
# the per-class reset ensures the module's non-controller classes aren't falsely flagged,
# while the full-visitor recurse finds the real controller inside the module.
class FlashTestCase < ActionController::TestCase
  class TestController < ActionController::Base
    def use_flash
      flash[:notice] = "hello"
      redirect_to root_path
    end
  end

  module ::Admin
    class NonController
      def helper
        flash[:alert] = "msg"
        render :index
      end
    end
  end
end

# Flash in def-with-rescue — no render in right siblings, only redirect
class PaymentsController < ApplicationController
  def create
    unless valid_payment?
      flash[:error] = "Invalid payment"
      redirect_to payments_path
      return
    end
    process_payment
    redirect_to receipts_path
  rescue PaymentError => e
    flash[:error] = e.message
    redirect_to payments_path
  end
end

# case branches with redirect_to in the same branch should not trigger
class VotesController < ApplicationController
  def create
    case params[:vote]
    when "0"
      flash[:notice] = "Against"
      redirect_to votes_path
    when "1"
      flash[:notice] = "For"
      redirect_to votes_path
    end
  end
end

# Direct flash in a begin body with rescue should not see render after the block
class CountryBandsController < ApplicationController
  def update
    begin
      flash[:success] = if has_anything_changed
                          "Successfully updated band data."
                        else
                          "No change to band data."
                        end
    rescue ActiveRecord::RecordInvalid => e
      flash[:danger] = e.message
    end
    render :edit
  end
end

# Direct flash in an explicit begin body should stay ignored even when the
# begin body itself renders and rescue also renders
class EnumerationsController < ApplicationController
  def create
    begin
      flash[:success] = "Created"
      render :plain => "Success"
    rescue
      flash.now[:error] = t("enumeration._frontend.messages.create_error")
      render_aspace_partial :partial => "new"
    end
  end
end

# FP guard: render inside an unless branch in a begin body should stay ignored
class TarUploadsController < ApplicationController
  def create_from_tar
    begin
      unless valid_course_tar(tar_extract)
        flash[:error] = "Invalid tarball"
        flash[:html_safe] = true
        render(action: "new") && return
      end
    rescue SyntaxError => e
      flash[:error] = e.message
    end
  end
end

# FP guard: nested respond_to inside an if branch should not see sibling format render
class SpentTimeController < ApplicationController
  def create
    if save_result
      flash[:notice] = l("time_entry_added_notice")
      logger.info("Everything went fine rendering report result")
      respond_to do |format|
        if current_date > to_date
          to_date
        elsif current_date < from_date
          from_date
        end
        format.html { redirect_to report_path }
        format.json { render json: time_entry, status: :created }
      end
    end
  end
end

# FP guard: case branches before redirect_to at method level should stay ignored
class MultifactorAuthsController < ApplicationController
  def update_level_and_redirect
    case level_param
    when "ui_and_api", "ui_and_gem_signin"
      flash[:success] = t("multifactor_auths.update.success")
      current_user.update(mfa_level: level_param)
    else
      flash[:error] = t("multifactor_auths.update.invalid_level")
    end

    redirect_to edit_settings_path
  end
end

# FP guard: multi-branch case with redirect_to after the case should stay ignored
class RegistrationsController < ApplicationController
  def payment_status
    case stored_intent_status
    when :succeeded
      record_success
    when :pending
      record_pending
    else
      flash[:error] = "Invalid PaymentIntent status"
    end

    redirect_to competition_register_path(competition_id)
  end
end
