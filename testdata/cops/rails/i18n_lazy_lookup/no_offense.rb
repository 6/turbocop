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
