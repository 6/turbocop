# nitrocop-filename: app/controllers/books_controller.rb
def show
  t("books.show.title")
  ^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLazyLookup: Use lazy lookup for i18n keys.
end
def create
  t("books.create.success")
  ^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLazyLookup: Use lazy lookup for i18n keys.
end
def edit
  translate("books.edit.name")
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLazyLookup: Use lazy lookup for i18n keys.
end
