#pragma once

namespace lazy_using_import_retains_base_method {

template<class T>
struct allocator {
  typedef T value_type;
  typedef T * pointer;
};

template<class Alloc>
struct allocator_traits {
  typedef typename Alloc::value_type value_type;
  typedef typename Alloc::pointer pointer;

  template<class U>
  using rebind_alloc = allocator<U>;
};

template<class Owner, class T, class Alloc>
struct pointer_layout {
  typedef Alloc allocator_type;
  typedef T * pointer;
  typedef const T * const_pointer;
  typedef pointer iterator;
  typedef const_pointer const_iterator;

  pointer first;

  pointer begin()
  {
    return first;
  }

  pointer begin() const
  {
    return first;
  }
};

template<class T, class Alloc, template<class, class, class> class Layout>
struct split_buffer : private Layout<split_buffer<T, Alloc, Layout>, T, Alloc> {
  typedef Layout<split_buffer<T, Alloc, Layout>, T, Alloc> base_type;
  typedef typename base_type::pointer pointer;
  typedef typename base_type::const_iterator const_iterator;

  split_buffer()
  {
  }

  ~split_buffer()
  {
  }

  using base_type::begin;
};

template<class T>
struct deque_like {
  template<class U, class Alloc>
  using split_buffer_alias = split_buffer<U, Alloc, pointer_layout>;

  typedef allocator<T> allocator_type;
  typedef allocator_traits<allocator_type> alloc_traits;
  typedef typename alloc_traits::pointer pointer;
  typedef typename alloc_traits::template rebind_alloc<pointer> pointer_allocator;
  typedef split_buffer_alias<pointer, pointer_allocator> map_type;
  typedef typename map_type::const_iterator map_const_iterator;

  map_type map;

  ~deque_like()
  {
    typename map_type::pointer it = map.begin();
    (void)it;
  }
};

}
