namespace api { namespace detail {
template<class T> struct box { typedef T type; };
} using namespace detail; }
template<class T> typename api::box<T>::type id(T);
decltype(id(1)) value;
