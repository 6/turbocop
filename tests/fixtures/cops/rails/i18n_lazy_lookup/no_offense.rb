# turbocop-filename: app/controllers/books_controller.rb
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
