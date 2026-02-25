User
^^^^ Lint/ConstantResolution: Fully qualify this constant to avoid possibly ambiguous resolution.
Login
^^^^^ Lint/ConstantResolution: Fully qualify this constant to avoid possibly ambiguous resolution.
Config
^^^^^^ Lint/ConstantResolution: Fully qualify this constant to avoid possibly ambiguous resolution.
# Unqualified root of a qualified superclass IS flagged (RuboCop does this too)
class MyService < Base::Service
                  ^^^^ Lint/ConstantResolution: Fully qualify this constant to avoid possibly ambiguous resolution.
end
