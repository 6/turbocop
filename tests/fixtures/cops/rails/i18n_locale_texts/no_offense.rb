validates :email, presence: { message: :email_missing }
redirect_to root_path, notice: t(".success")
flash[:notice] = t(".success")
mail(to: user.email)
mail(to: user.email, subject: t("mailers.users.welcome"))
validates :name, presence: true
