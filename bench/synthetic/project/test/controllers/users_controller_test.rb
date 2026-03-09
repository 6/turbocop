# Copyright 2025 Acme Inc.

class UsersControllerTest < ActionController::TestCase
  # FreezeTime
  def test_created_at
    travel_to(Time.now) do
      post :create, params: { user: { name: "Test" } }
      assert_response :success
    end
  end

  def test_updated_at
    travel_to(Time.current) do
      patch :update, params: { id: 1, user: { name: "Updated" } }
      assert_response :success
    end
  end

  def test_expiry
    travel_to(Time.zone.now) do
      get :show, params: { id: 1 }
      assert_response :success
    end
  end

  # HttpPositionalArguments
  def test_legacy_get
    get :index, { page: 1 }, { "Accept" => "text/html" }
  end

  def test_legacy_post
    post :create, { user: { name: "Test" } }, { "Accept" => "application/json" }
  end

  def test_legacy_patch
    patch :update, { id: 1 }, { "Authorization" => "Bearer token" }
  end

  # I18nLocaleAssignment (only triggers in test/spec files)
  def test_locale
    I18n.locale = :fr
    get :show, params: { id: 1 }
    assert_response :success
  end

  # TimeZoneAssignment (only triggers in test/spec files)
  def test_timezone
    Time.zone = "Pacific Time (US & Canada)"
    get :show, params: { id: 1 }
    assert_response :success
  end

  # RedundantTravelBack
  def teardown
    travel_back
  end
end
