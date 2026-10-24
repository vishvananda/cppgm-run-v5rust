// GNU vector identity survives attribute positions, qualification and substitution.
using size_t = decltype(sizeof(0));
typedef int words __attribute__((vector_size(16)));
using same [[gnu::vector_size(16)]] = int;
using floats __attribute__((ext_vector_type(3))) = float;
using flags __attribute__((ext_vector_type(17))) = bool;
template<class T, size_t N>
using lanes __attribute__((ext_vector_type(N))) = T;
template<class T, size_t N>
using bytes __attribute__((vector_size(N))) = T;
template<class T, size_t N> constexpr size_t count(lanes<T,N> const*) { return N; }
template<class T, size_t N> constexpr size_t width(bytes<T,N> const*) { return N; }
static_assert(__is_same(words, same), "canonical attribute identity");
static_assert(!__is_same(words, int[4]), "not an array");
static_assert(__is_same(lanes<const int,4>, words), "substituted lane is unqualified");
static_assert(__is_aggregate(words), "vector aggregate property");
static_assert(__is_same(lanes<int,4>, bytes<int,16>), "byte and lane counts");
static_assert(sizeof(floats) == 16 && alignof(floats) == 16, "rounded layout");
static_assert(sizeof(flags) == 4 && alignof(flags) == 4, "packed bool lanes");
static_assert(sizeof(lanes<short,3>) == 8, "dependent lanes");
static_assert(sizeof(bytes<double,32>) == 32, "dependent bytes");
static_assert(__is_const(const words) && !__is_array(words), "vector cv and shape");
static_assert(__is_trivially_copyable(words) && __is_literal_type(words), "properties");
static_assert(count((words const*)0) == 4, "lane deduction");
static_assert(width((words const*)0) == 16, "byte deduction");
struct Holder { words value; int tail; };
static_assert(sizeof(Holder) == 32 && alignof(Holder) == 16, "member layout");
inline words literal() { return words{1,2,3}; }
inline void type_id() { (void)((__attribute__((vector_size(16))) int){1,2,3,4}); }
int anchor;
