// VALIDATION: compile-pass
// A substituted default type argument keeps its semantic identity when its
// generated spelling is shadowed by an enclosing template parameter.

template<class T>
struct decay
{
  typedef T type;
};

template<class T>
using decay_t = typename decay<T>::type;

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

struct R {};

template<class Value, class Owner>
struct accepts
{
  static const bool value = false;
};

template<>
struct accepts<::R, void>
{
  static const bool value = true;
};

template<class R>
struct wrapper
{
  int selected;

  template<class F,
           class Value = decay_t<F>,
           typename enable_if<accepts<Value, R>::value, int>::type = 0>
  wrapper(F &&) : selected(7) {}
};

int main()
{
  wrapper<void> value(R{});
  return value.selected == 7 ? 0 : 1;
}
