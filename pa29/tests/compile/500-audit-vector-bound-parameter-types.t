template<class T, int N> using lanes __attribute__((ext_vector_type(N))) = T;
template<class T, short N> using bytes __attribute__((vector_size(N))) = T;
template<class T, int N, class U>
constexpr int count(lanes<T,N> const*, U) { return N; }
template<class T, short N>
constexpr int width(bytes<T,N> const*) { return N; }
typedef int words __attribute__((vector_size(16)));
static_assert(count((words const*)0, 0) == 4, "deduced lane count converts to int");
static_assert(width((words const*)0) == 16, "deduced byte count converts to short");
int anchor;
