// VALIDATION: compile-pass
// Once a namespace function template has concrete arguments, its return-type
// substitution must not inherit unrelated placeholders from the class-template
// specialization that called it.  The second overload is ill-formed after
// substitution and must be discarded instead of making the call ambiguous.

template<int Value>
struct bool_constant {};

template<class Condition, class = bool_constant<1> >
struct enable_if_and;

template<class T>
struct remove_cvref;

template<class... Allocators>
struct allocator_adaptor {};

template<class T>
struct uses_allocator;

namespace detail {

struct is_not_pair : bool_constant<1> {};

template<class ConstructAllocator, class AllocatorArg, class T, class... Args>
void dispatch(ConstructAllocator, AllocatorArg, T *, Args...)
{}

template<class ConstructAllocator, class AllocatorArg, class T, class... Args>
typename enable_if_and<
    is_not_pair,
    uses_allocator<typename remove_cvref<AllocatorArg>::type> >::type
dispatch(ConstructAllocator, AllocatorArg, T *, Args...)
{}

template<class OuterAllocator, class... InnerAllocators>
struct adaptor_base
{
  OuterAllocator &outer_allocator()
  {
    return allocator;
  }

  OuterAllocator allocator;
};

}

template<class OuterAllocator, class... InnerAllocators>
struct allocator_adaptor<OuterAllocator, InnerAllocators...>
  : detail::adaptor_base<OuterAllocator, InnerAllocators...>
{
  template<class T, class... Args>
  void construct(T *output, Args... args)
  {
    detail::dispatch(this->outer_allocator(), 0, output,
                     static_cast<Args>(args)...);
  }
};

int main()
{
  int value = 0;
  allocator_adaptor<int> allocator;
  allocator.construct(&value);
  return value;
}
