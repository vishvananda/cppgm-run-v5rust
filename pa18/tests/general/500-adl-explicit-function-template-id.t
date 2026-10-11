// VALIDATION: compile-pass

namespace lib
{
  typedef decltype(sizeof(0)) size_t;
  static const size_t default_alignment = 0;

  template<class T,
           class P = long,
           class O = unsigned long,
           size_t A = default_alignment>
  struct ptr;

  template<class T, class P, class O, size_t A>
  struct ptr
  {
    ptr()
    {
    }

    template<class U>
    ptr(const ptr<U, P, O, A> &)
    {
    }

    ptr & operator=(const ptr &)
    {
      return *this;
    }
  };

  template<class T, class P, class O, size_t A, class U>
  ptr<T, P, O, A> static_pointer_cast(const ptr<U, P, O, A> & r)
  {
    return ptr<T, P, O, A>(r);
  }
}

using namespace lib;

int main()
{
  ptr<void> source;
  ptr<int> target;
  target = static_pointer_cast<int>(source);
  return 0;
}
