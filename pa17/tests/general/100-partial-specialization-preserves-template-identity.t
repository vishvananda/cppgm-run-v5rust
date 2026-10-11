namespace a { template<int> struct item {}; }
namespace b {
template<int> struct item {};
template<class> struct trait { enum { value = 0 }; };
template<int I> struct trait<item<I> > { enum { value = 1 }; };
}
static_assert(!b::trait<a::item<0> >::value, "");
