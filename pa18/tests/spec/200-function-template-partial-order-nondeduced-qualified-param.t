// N3485 focus: 14.8.2.5 [temp.deduct.type], 14.8.3 [temp.over]
// A dependent qualified member type is a non-deduced context during partial
// ordering; it must not hide the more-specialized nested template-id pattern.

template<class A, class B, class C>
struct local_map
{
  typedef typename B::key_type key_type;
};

template<class T>
struct property_traits
{
  typedef typename T::key_type key_type;
};

struct global_map
{
  typedef unsigned long key_type;
};

template<class T>
struct wrapper
{};

struct selected_choice
{};

struct generic_choice
{};

template<class T>
generic_choice select(wrapper<T> const &,
                      typename property_traits<T>::key_type,
                      int)
{
  return generic_choice();
}

template<class A, class B, class C>
selected_choice select(wrapper<local_map<A, B, C> > const &,
                       typename property_traits<B>::key_type,
                       int)
{
  return selected_choice();
}

int main()
{
  wrapper<local_map<int, global_map, long> > value;
  selected_choice result = select(value, 0UL, 0);
  (void)result;
  return 0;
}
