// VALIDATION: compile-pass
// A member template instantiated from a class partial specialization may retain
// source syntax for the specialization's owner pack.  Function-template
// deduction must keep the concrete typed pack instead of replacing it with the
// old dependent placeholder.

template<class... Allocators>
struct allocator_adaptor;

template<class OuterAllocator, class... InnerAllocators>
struct allocator_adaptor<OuterAllocator, InnerAllocators...>
{
  allocator_adaptor()
  {}

  template<class OtherOuter>
  allocator_adaptor(
      const allocator_adaptor<OtherOuter, InnerAllocators...> &)
  {}
};

struct outer_one {};
struct outer_two {};
struct inner {};

template<class T>
T &&declval();

typedef allocator_adaptor<outer_two, inner> source_type;
typedef allocator_adaptor<outer_one, inner> target_type;

template<class T>
auto probe(int) -> decltype(T(declval<const source_type &>()), char());

template<class>
long probe(...);

int main()
{
  return sizeof(probe<target_type>(0)) == sizeof(char) ? 0 : 1;
}
