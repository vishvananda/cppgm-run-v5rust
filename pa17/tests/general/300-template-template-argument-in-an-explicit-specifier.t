// A template template argument is spelled exactly like a type argument, so the
// walker that interns a function template's result identity cannot tell them
// apart and asks whether each argument names a concrete type.  A bare
// template-name does not, and that answer has to be reported rather than
// thrown: the walker already asks in the mode that reports a failure.
//
// Answering it reaches a second defect behind the first.  One of the two
// places that wire an instantiated definition carried the body without the
// constructor's mem-initializers, so the constructor ran and initialized
// nothing -- which is why `tag` is checked here and not just the compile.
//
// libc++'s tuple writes the shape as
// explicit(_Not<_Lazy<_And, _IsImpDefault<_Tp>...> >::value).

template <class T>
struct is_small
{
  static const bool value = sizeof(T) <= 4;
};

template <class... T>
struct all_of;

template <>
struct all_of<>
{
  static const bool value = true;
};

template <class H, class... R>
struct all_of<H, R...>
{
  static const bool value = is_small<H>::value && all_of<R...>::value;
};

template <template <class...> class F, class... A>
struct lazy
{
  static const bool value = F<A...>::value;
};

template <class... T>
struct box
{
  int tag;

  // `all_of` is a template template argument, and the specifier belongs to a
  // constructor template, which is what puts it on the walker's path.
  template <class U = int>
  explicit(lazy<all_of, T...>::value) box() : tag(7) { }
};

int main()
{
  // The specifier is true here and false below; both constructors have to run
  // their mem-initializer.
  box<int> small_box;
  box<double> large_box;
  return (small_box.tag == 7 && large_box.tag == 7) ? 0 : 1;
}
