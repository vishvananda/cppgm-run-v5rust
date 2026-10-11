template<bool, class T>
struct EnableIf {
};

template<class T>
struct EnableIf<true, T> {
  typedef T type;
};

template<class From, class To>
struct Compatible {
  static const bool value = false;
};

template<class From, class To>
struct Compatible<From *, To *> {
  static const bool value = true;
};

template<class T>
struct SharedLike {
  template<class Y, class Result = void>
  using CompatibleAlias =
      typename EnableIf<Compatible<Y *, T *>::value, Result>::type;

  template<class Y>
  using Assignable = CompatibleAlias<Y, SharedLike &>;

  typedef Assignable<T> SelfAssignable;
};

int main()
{
  return sizeof(SharedLike<int>) > 0 ? 0 : 1;
}
