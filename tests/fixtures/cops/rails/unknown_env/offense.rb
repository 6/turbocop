Rails.env.staging?
^^^^^^^^^^^^^^^^^^ Rails/UnknownEnv: Unknown environment `staging`.
Rails.env.qa?
^^^^^^^^^^^^^ Rails/UnknownEnv: Unknown environment `qa`.
Rails.env.preprod?
^^^^^^^^^^^^^^^^^^ Rails/UnknownEnv: Unknown environment `preprod`.
::Rails.env.staging?
^^^^^^^^^^^^^^^^^^^^ Rails/UnknownEnv: Unknown environment `staging`.
Rails.env.local?
^^^^^^^^^^^^^^^^ Rails/UnknownEnv: Unknown environment `local`.
::Rails.env.local?
^^^^^^^^^^^^^^^^^^ Rails/UnknownEnv: Unknown environment `local`.
Rails.env == 'staging'
             ^^^^^^^^^ Rails/UnknownEnv: Unknown environment `staging`.
Rails.env == "profile"
             ^^^^^^^^^ Rails/UnknownEnv: Unknown environment `profile`.
'staging' == Rails.env
^^^^^^^^^ Rails/UnknownEnv: Unknown environment `staging`.
Rails.env === 'unknown_thing'
              ^^^^^^^^^^^^^^^ Rails/UnknownEnv: Unknown environment `unknown_thing`.
::Rails.env == 'staging'
               ^^^^^^^^^ Rails/UnknownEnv: Unknown environment `staging`.
