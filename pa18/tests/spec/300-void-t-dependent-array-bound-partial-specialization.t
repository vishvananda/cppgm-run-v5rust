template<class...> struct voider { typedef void type; };
template<class... T> using void_t = typename voider<T...>::type;

template<class T, class = void>
struct is_complete { static const bool value = false; };

template<class T>
struct is_complete<T, void_t<int[sizeof(T)]> >
{ static const bool value = true; };

struct complete {};
struct incomplete;

static_assert(is_complete<complete>::value, "complete");
static_assert(!is_complete<incomplete>::value, "incomplete");

int main() { return 0; }
