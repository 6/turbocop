# nitrocop-filename: app/controllers/books_controller.rb
class BooksController < ApplicationController
  I18n.t("users.show.title")
  I18n.t("simple_key")
  t(".title")
  t("hello")
  t(:symbol_key)
  t("one.two")
  t("admin.reports.processed_msg")
  # Key doesn't match the enclosing method name
  def validate_token
    t("books.show.token_failure")
  end
  # Key has extra segments between action and final key — not a lazy lookup candidate
  def destroy
    t("books.destroy.flash.logged_out_successfully")
  end
  def create
    t("books.create.notices.saved_msg")
  end
  # Private methods should not be flagged
  private

  def require_cookies
    t("books.require_cookies.cookies_needed")
  end

  protected

  def set_locale
    t("books.set_locale.label")
  end

  public

  # Symbol keys should not be flagged in lazy mode — RuboCop only flags string keys
  def index
    t(:'books.index.page_title')
  end
  def update
    t(:'books.update.submitted')
  end
end
