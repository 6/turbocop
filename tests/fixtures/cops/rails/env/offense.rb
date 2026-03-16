Rails.env.production?
^^^^^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
Rails.env.development? || Rails.env.test?
                          ^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
^^^^^^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
Rails.env.staging?
^^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
if Rails.env.local?
   ^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
  do_something
end
::Rails.env.production?
^^^^^^^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
x = ::Rails.env.staging?
    ^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
::Rails.env.development? || ::Rails.env.test?
                            ^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
%w[test development].member?(Rails.env)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
@envs.any?(Rails.env)
^^^^^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
env = Rails.env.to_s if defined?(Rails.env)
                        ^^^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
@default_env = Rails.env if defined? Rails.env
                            ^^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
@rails_env = defined?(::Rails.env) ? Rails.env.to_s : 'shards'
             ^^^^^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
if defined?(Rails.env)
   ^^^^^^^^^^^^^^^^^^^ Rails/Env: Use Feature Flags or config instead of `Rails.env`.
  Rails.env
end
