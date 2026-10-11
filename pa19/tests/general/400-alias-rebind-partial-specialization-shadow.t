namespace std
{
  template<class _Tp>
  struct allocator
  {
    typedef _Tp value_type;
    allocator() {}

    template<class _Up>
    allocator(const allocator<_Up>&) {}
  };

  template<class _Tp, class _Up, class = void>
  struct __has_rebind_other
  {
    static const bool value = false;
  };

  template<class _Tp,
           class _Up,
           bool = __has_rebind_other<_Tp, _Up>::value>
  struct __allocator_traits_rebind
  {
    using type =
        typename _Tp::template rebind<_Up>::other;
  };

  template<template<class, class...> class _Alloc,
           class _Tp,
           class... _Args,
           class _Up>
  struct __allocator_traits_rebind<_Alloc<_Tp, _Args...>, _Up, false>
  {
    using type = _Alloc<_Up, _Args...>;
  };

  template<class _Alloc, class _Tp>
  using __allocator_traits_rebind_t =
      typename __allocator_traits_rebind<_Alloc, _Tp>::type;

  template<class _Alloc>
  struct allocator_traits
  {
    using allocator_type = _Alloc;
    using value_type = typename allocator_type::value_type;

    template<class _Tp>
    using rebind_alloc =
        __allocator_traits_rebind_t<allocator_type, _Tp>;
  };
}

int main()
{
  std::allocator<int> ia;
  std::allocator_traits<std::allocator<int> >::rebind_alloc<void*> ra(ia);
  std::allocator<void*>* pva = &ra;
  (void)pva;
  return 0;
}
