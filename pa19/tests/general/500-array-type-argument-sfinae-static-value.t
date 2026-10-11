// Reduced from Boost.Bloom filter_core. A substitution probe for a class partial
// specialization must reuse the bound class-template argument type instead of
// falling back to reparsing generated text such as "unsigned int [4]".
typedef decltype(sizeof(0)) size_t;

template<bool B, class T = void>
struct enable_if {
};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<class T, class = void>
struct used_value_size {
  static const size_t value = sizeof(typename T::value_type);
};

template<class T>
struct used_value_size<T, typename enable_if<(T::used_value_size != 0)>::type> {
  static const size_t value = T::used_value_size;
};

template<class Block, size_t K>
struct block {
  static const size_t k = K;
  typedef Block value_type;
};

template<size_t K, class Subfilter>
struct filter_core {
  typedef Subfilter subfilter;
  typedef typename subfilter::value_type block_type;
  static const size_t used = used_value_size<subfilter>::value;
  static const size_t block_size = sizeof(block_type);
  static const size_t prefetched = 1 + block_size / used;
};

static_assert(filter_core<1, block<unsigned int[4], 4> >::prefetched == 2, "");
