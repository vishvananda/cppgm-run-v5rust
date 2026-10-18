// expect exit 0
// PA18: only an own member suffix replaces the enclosing object context.
template<class A, class B> struct same { static const bool value = false; };
template<class A> struct same<A, A> { static const bool value = true; };

struct Object {
  auto plain() -> auto (*)(int) -> decltype(this);
  template<class T> auto nested(T) const -> auto (*)(T) -> decltype(this);

  int local() const {
    struct Local {
      static auto from_static() -> decltype(this);
      friend auto from_friend() -> decltype(this);
      auto own() -> decltype(this) { return this; }
    };
    static_assert(same<decltype(Local::from_static()), const Object*>::value,
                  "nonmember suffix retains the enclosing body");
    Local object;
    return object.own() == &object ? 0 : 1;
  }
};

static_assert(same<decltype(((Object*)0)->plain()), Object* (*)(int)>::value,
              "nested abstract suffix retains the outer signature");
static_assert(same<decltype(((const Object*)0)->nested(0)), const Object* (*)(int)>::value,
              "nested template signature retains cv and substitutes once");

int main() {
  const Object object = {};
  return object.local();
}
