int index;

namespace n {
template<class T> struct base {};
}

template<class Tag, class BimapType>
struct view : n::base<
  typename BimapType::core_type::template index<Tag>::type
> {};
