// N3485 focus: [temp.dep.type] current instantiation names in member alias templates.
namespace ns {
namespace {
struct Block {};
}

template<class T>
struct Alloc {
  typedef T value_type;
};

template<class A, class T, bool B>
struct Rebind;

template<template<class> class AllocT, class Old, class T>
struct Rebind<AllocT<Old>, T, false> {
  typedef AllocT<T> type;
};

template<class A>
struct Traits {
  typedef A allocator_type;
  typedef typename allocator_type::value_type value_type;
  typedef value_type* pointer;

  template<class T>
  using rebind_alloc = typename Rebind<allocator_type, T, false>::type;

  template<class T>
  using rebind_traits = Traits<rebind_alloc<T> >;
};
}

ns::Traits<ns::Alloc<ns::Block> >::pointer p;

int main() { return 0; }
