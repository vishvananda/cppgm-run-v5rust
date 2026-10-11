// Reduced from Boost.FunctionTypes interface_example. A class that is first
// collected for reference members must not preserve ordinary in-class member
// class templates across the later full collection; they need to be rebuilt
// with the complete owner scope.

namespace example {
template<class Info>
struct vtable_entry {
  typedef typename Info::result (*type)(void *, typename Info::param);
};

template<class Info>
struct member {
  typedef typename Info::context context;
  typedef typename Info::result (context::*mem_func_ptr)(typename Info::param) const;

  template<mem_func_ptr MemFuncPtr>
  static typename Info::result wrap(void * c)
  {
    return (reinterpret_cast<context *>(c)->*MemFuncPtr)();
  }

  template<mem_func_ptr MemFuncPtr, class T0>
  static typename Info::result wrap(void * c, T0 a0)
  {
    return (reinterpret_cast<context *>(c)->*MemFuncPtr)(a0);
  }
};
}

class interface_x {
  struct vtable {
    template<class T = void *>
    struct info {
      typedef void result;
      typedef T context;
      typedef int param;
    };

    example::vtable_entry<info<> >::type func;
  };

  vtable const * ptr_vtable;
  void * ptr_that;

  template<class T>
  struct holder {
    static vtable const table;
  };

public:
  template<class T>
  interface_x(T & that)
    : ptr_vtable(&holder<T>::table),
      ptr_that(&that)
  {}

  void call(int value) const
  {
    ptr_vtable->func(ptr_that, value);
  }
};

template<class T>
interface_x::vtable const interface_x::holder<T>::table = {
  &example::member<interface_x::vtable::info<T> >::template wrap<&T::call>
};

struct target {
  void call(int) const {}
};

int main()
{
  target value;
  interface_x iface(value);
  iface.call(1);
  return 0;
}
