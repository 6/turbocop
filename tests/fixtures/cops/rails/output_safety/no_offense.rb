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
