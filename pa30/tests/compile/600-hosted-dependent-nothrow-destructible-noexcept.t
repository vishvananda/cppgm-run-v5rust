#include <type_traits>

template<class T>
struct allocator_like
{
};

template<class T>
struct traits_like;

template<class T>
struct traits_like<allocator_like<T> >
{
  template<class U>
  static void destroy(allocator_like<T>&, U*)
      noexcept(std::is_nothrow_destructible<U>::value)
  {
  }
};

template<class T>
void destroy_one(allocator_like<T>& allocator, T* pointer)
    noexcept(noexcept(traits_like<allocator_like<T> >::destroy(
        allocator, pointer)))
{
  traits_like<allocator_like<T> >::destroy(allocator, pointer);
}

int main()
{
  allocator_like<unsigned> allocator;
  unsigned* pointer = 0;
  static_assert(noexcept(destroy_one(allocator, pointer)),
                "scalar destruction keeps the dependent wrapper non-throwing");
  destroy_one(allocator, pointer);
}
