validates :email, presence: { message: :email_missing }
redirect_to root_path, notice: t(".success")
flash[:notice] = t(".success")
mail(to: user.email)
mail(to: user.email, subject: t("mailers.users.welcome"))
validates :name, presence: true

# FP fix: multi-line string literals are parsed as `dstr` by the Parser gem,
# so RuboCop's `$str` NodePattern does not match them.
flash[:error] = "Your course has been disabled by your instructor.
                 Please contact them directly if you have any questions"
flash[:error] = "You cannot view this submission.
        Either an exam is in progress or this is an exam submission."
flash[:error] = "Errors found in tarball: Course name is invalid. Valid course names consist
            of letters, numbers, and hyphens, starting and ending with a letter or number."
redirect_to root_path, notice: "line one
line two"
validates :email, presence: { message: "line one
line two" }
mail(to: user.email, subject: "line one
line two")

# FP fix: flash as a local variable should not be flagged
# (RuboCop only matches `flash` as a method call, not a local variable)
flash = {}
flash[:error] = "This should not be flagged"
flash[:notice] = "Not flagged when flash is a local var"

# FP fix: flash(:category) with arguments is Scorched-specific, not Rails flash
# RuboCop's pattern matches (send nil? :flash) which means NO arguments
flash(:animals)[:cat] = 'meow'
flash(:names)[:jeff] = 'male'
