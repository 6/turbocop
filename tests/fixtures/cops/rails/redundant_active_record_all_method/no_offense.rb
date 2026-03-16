User.where(active: true)
User.order(:name)
User.all
User.all.each { |u| u.save }
User.find(1)
all.select { |role| role.can?(:manage) }
User.all.select(&:active?)
User.all.map(&:do_something)
User.all.select { |item| item.do_something }
User.all.any? { |item| item.do_something }
User.all.count { |item| item.do_something }
User.all.find { |item| item.do_something }
User.all.none? { |item| item.do_something }
User.all.one? { |item| item.do_something }
User.all.sum { |item| item.do_something }
page.all(:parameter).select(some_filter)
page.all(:parameter)
page.all(:parameter).do_something
user.articles.all.delete_all
user.articles.all.destroy_all
users.all.delete_all
all.delete_all
all.destroy_all
User.all()
User.all().do_something
all.where(active: true)
all.order(:name)
all.first
all.find_by(name: name)
all.includes(:articles)
class User
  def do_something
    all.where(id: ids)
  end
end
class User < Foo
  def do_something
    all.where(id: ids)
  end
end
