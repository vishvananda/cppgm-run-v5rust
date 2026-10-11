namespace algebra
{
  template<bool B, class Type>
  struct enable_if
  {
  };

  template<class Type>
  struct enable_if<true, Type>
  {
    typedef Type type;
  };

  template<class Type>
  struct is_good
  {
    static const bool value = true;
  };

  template<class Type>
  struct equality
  {
    typedef bool (type)(const Type &, const Type &);
  };

  template<class Type>
  typename enable_if<is_good<Type>::value, bool>::type
  is_distinct_equal(const Type &, const Type &)
  {
    return true;
  }

  template<class TypeA, class TypeB, class TypeC>
  void check_partial(typename equality<TypeA>::type * equal,
                     const TypeA & a,
                     const TypeB & b,
                     const TypeC & c)
  {
    (void)equal;
    (void)a;
    (void)b;
    (void)c;
  }
}

using namespace algebra;

template<class T>
void run_check()
{
  T a = T();
  check_partial(is_distinct_equal, a, a, a);
}

int main()
{
  run_check<int>();
  return 0;
}
