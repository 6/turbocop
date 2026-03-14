sanitize(content)
safe_method(str)
ERB::Util.html_escape(value)
CGI.escapeHTML(text)
content_tag(:p, text)
# String literal receivers are exempt
"<b>bold</b>".html_safe
''.html_safe
'safe string'.html_safe
# i18n method calls are exempt
t('key').html_safe
I18n.t('some.key', name: user.name).html_safe
translate('key').html_safe
I18n.translate('key').html_safe
# raw() with i18n argument is exempt
raw t('.owner_html', owner: user)
raw I18n.t('key')
raw translate('some.key')
# Static heredoc receivers are exempt
<<~HTML.html_safe
  <p>static content</p>
HTML
<<~TEXT.html_safe
  line one
  line two
TEXT
# i18n nested in method chain arguments (deep search)
some_helper(t('key')).html_safe
format_text(I18n.t('msg'), 'extra').html_safe
# safe_concat with string literal receiver
"<b>bold</b>".safe_concat(content)
# safe_concat with i18n in argument
out.safe_concat(t('key'))
buffer.safe_concat(I18n.translate('msg'))

# i18n inside keyword hash argument should suppress offense
# (RuboCop's def_node_search :i18n_method? searches ALL descendants including hash values)
raw(cell("component", context: { placeholder: t("key") }))
some_helper(options: { label: I18n.t("label") }).html_safe
