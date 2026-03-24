validates :email, presence: { message: "must be present" }
                                       ^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

redirect_to root_path, notice: "Post created!"
                               ^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

mail(to: user.email, subject: "Welcome to My Awesome Site")
                              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

flash[:notice] = "Post created!"
                 ^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

flash.now[:alert] = "Something went wrong!"
                    ^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

# FN fix: validates with message at top level (not nested in hash)
Topic.validates :title, confirmation: true, message: "Y U NO CONFIRM"
                                                     ^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

# FN fix: redirect_to with flash: { notice: "string" }
redirect_to root_path, flash: { notice: "User not found" }
                                        ^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

# FN fix: redirect_to with notice passed to URL helper (recursive search)
redirect_to admin_orders_url(notice: "Order was successfully created.")
                                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

# FN fix: mail with subject in a hash literal argument
mail({ subject: "The first email on new API!" }.merge!(hash))
                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

# FN fix: redirect_to with explicit hash containing flash alert
redirect_to [:admin, @edition], { flash: { alert: "This is historic content" } }
                                                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

# FN fix: redirect_to with splatted ternary containing alert
redirect_to root_path, **(condition ? { warning: "Text" } : { alert: "Other" })
                                                                     ^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

# FN fix: backslash line continuation strings don't contain actual newlines
flash[:error] =
  "Please specify a valid image name comprised of lowercase letters, digits, and \
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.
   underscores. You may use one forward slash separator."

# FN fix: single-line string with \r\n escape sequences should still be flagged
# (Parser gem treats these as `str` since they are single-line in source)
flash['info'] = "Thank you!\r\nWe appreciate your feedback."
                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.
