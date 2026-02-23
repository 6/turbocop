validates :email, presence: { message: "must be present" }
                                       ^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

redirect_to root_path, notice: "Post created!"
                               ^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

mail(to: user.email, subject: "Welcome to My Awesome Site")
                              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.

flash[:notice] = "Post created!"
                 ^^^^^^^^^^^^^^^ Rails/I18nLocaleTexts: Move locale texts to the locale files in the `config/locales` directory.
