// Reduced from Boost.FunctionTypes fast_mem_fn. A class partial
// specialization whose argument is a cv-qualified member-function pointer must
// deduce an empty parameter pack from the function parameter list.

namespace meta {

template<bool B>
struct bool_constant {
  static const bool value = B;
};

template<bool B, class T = void>
struct enable_if_c {
  typedef T type;
};

template<class T>
struct enable_if_c<false, T> {};

template<class Cond, class T = void>
struct enable_if : enable_if_c<Cond::value, T> {};

template<class T>
struct is_const_member_function_pointer : bool_constant<false> {};

template<class Ret, class C, class... Args>
struct is_const_member_function_pointer<Ret (C::*)(Args...) const>
  : bool_constant<true> {};

}

namespace example {

template<class MFPT>
struct maker {
  int value() const { return 0; }
};

template<class MFPT>
typename meta::enable_if<meta::is_const_member_function_pointer<MFPT>,
                         maker<MFPT> >::type
make(MFPT)
{
  return maker<MFPT>();
}

}

class target {
public:
  int id() const { return 0; }
};

template<class F>
int wrapper(target const & value, F fn)
{
  return fn.value() + value.id();
}

int main()
{
  target value;
  return wrapper(value, ::example::make(&target::id));
}
