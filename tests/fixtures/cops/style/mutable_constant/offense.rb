CONST = []
        ^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST2 = {}
         ^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST3 = "hello"
         ^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

# ||= assignment is also flagged
CONST4 ||= [1, 2, 3]
           ^^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST5 ||= { a: 1, b: 2 }
           ^^^^^^^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST6 ||= 'str'
           ^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

# %w and %i array literals
CONST7 = %w[a b c]
         ^^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST8 = %i[a b c]
         ^^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST9 = %w(foo bar)
         ^^^^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

# Heredoc is mutable
CONST10 = <<~HERE
          ^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.
  some text
HERE

CONST11 = <<~RUBY
          ^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.
  code here
RUBY

# Module::CONST ||= value
Mod::CONST12 ||= [1]
                 ^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

# Backtick (xstring) literals are mutable
CONST13 = `uname`
          ^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

CONST14 = `echo hello`
          ^^^^^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.

BLOCKED_KEYWORDS =
  'data|at will|equal|status|eligibility|analysis|300 log|delayed' \
  ^ Style/MutableConstant: Freeze mutable objects assigned to constants.
  '|(histor)(y|ies)' \
  "|#{Date.current.year.to_s}" \
  "|#{BLOCKED_PHRASES}" \
  "|(#{SIMPLE_SINGULARS.join('|')})s?"

ENTERPRISE_VERIFICATION_URL =
  "https://recaptchaenterprise.googleapis.com/v1/projects/#{GOOGLE_CLOUD_PROJECT_ID}/" \
  ^ Style/MutableConstant: Freeze mutable objects assigned to constants.
  "assessments?key=#{GlobalConfig.get("ENTERPRISE_RECAPTCHA_API_KEY")}"

INVALID_ACCESS_KEY_ID_FORMAT =
  "invalid 'access_key_id' parameter format. The access key ID must be a " \
  ^ Style/MutableConstant: Freeze mutable objects assigned to constants.
  "valid AWS access key ID. The valid format is: " \
  "#{ACCESS_KEY_ID_FIELD_VALID_FORMAT}"

INVALID_SECRET_ACCESS_KEY_FORMAT =
  "invalid 'secret_access_key' parameter format. The secret access key " \
  ^ Style/MutableConstant: Freeze mutable objects assigned to constants.
  "must be a valid AWS secret access key. The valid format is: " \
  "#{SECRET_ACCESS_KEY_FIELD_VALID_FORMAT}"

ETCD_URL = "https://github.com/coreos/etcd/releases/download/" \
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.
           "#{ETCD_VERSION}/etcd-#{ETCD_VERSION}-linux-amd64.tar.gz"

FILE_PATH = __FILE__
            ^^^^^^^^ Style/MutableConstant: Freeze mutable objects assigned to constants.
