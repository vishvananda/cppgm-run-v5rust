// Reduced from Boost.FunctionTypes fast_mem_fn. The selected class partial
// specialization has a non-type member-function pointer parameter whose type
// depends on an earlier type parameter; the member pointer value must remain
// available inside a member template body.

class target {
public:
  int id() const { return 7; }
};

namespace example {

template<class MFPT, MFPT MemberFunction, int Arity = 1>
struct fast_mem_fn;

template<class MFPT, MFPT MemberFunction>
struct fast_mem_fn<MFPT, MemberFunction, 1> {
  template<class T>
  int operator()(T const & value) const
  {
    return (value.*MemberFunction)();
  }
};

template<class MFPT>
struct fast_mem_fn_maker {
  template<MFPT Callee>
  fast_mem_fn<MFPT, Callee> make_fast_mem_fn()
  {
    return fast_mem_fn<MFPT, Callee>();
  }
};

}

template<class Function>
struct holder {
  Function fn;

  explicit holder(Function value) : fn(value) {}

  template<class T>
  int apply(T const & value) const
  {
    return fn(value);
  }
};

template<class Function>
holder<Function> make_holder(Function value)
{
  return holder<Function>(value);
}

int main()
{
  target value;
  example::fast_mem_fn_maker<int (target::*)() const> maker;
  return make_holder(maker.make_fast_mem_fn<&target::id>()).apply(value) == 7 ? 0 : 1;
}
