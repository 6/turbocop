foo && foo.bar
^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

foo && foo.bar(param1, param2)
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

foo && foo.bar.baz
^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

foo && foo.nil?
^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

foo.nil? ? nil : foo.bar
^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

foo ? foo.bar : nil
^^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

!foo.nil? ? foo.bar : nil
^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

callback.call unless callback.nil?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

handler.process unless handler.nil?
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

obj.bar if obj
^^^^^^^^^^^^^^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

if data
^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.
  data.each do |k, v|
  end
end

after_save { if user then user.update_contribution_count end }
             ^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

after_destroy { if user then user.update_contribution_count end }
                ^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

return self[:postmark_template_alias] && self[:postmark_template_alias].to_s if val.nil?
       ^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

mail.cc && (mail.cc.include? 'support@agileventures.org')
^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

if fd && fd.respond_to?(:each)
   ^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.
  fd.each {}
end

if new_model_collection and new_model_collection.is_a?(Array)
   ^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.
  new_model_collection.each {}
end

(other.class == Path) && geometry.equals(other && other.respond_to?(:geometry) && other.geometry)
                                         ^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

proof && dom_body && dom_body.include?( proof )
^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

proof && response && response.include?( proof )
^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

uri.port = port ? port.to_i : nil
^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

uri.port = port ? port.to_i : nil
^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

user_is_current_user && record.campaign && record.campaign.users_can_join?
^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.

user_is_current_user && record.campaign && record.campaign.dms_can_join?
^ Style/SafeNavigation: Use safe navigation (`&.`) instead of checking if an object exists before calling the method.
