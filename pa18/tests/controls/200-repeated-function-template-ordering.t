// N3485 14.8.2.4: partial ordering applies independently to calls and
// addresses, including different argument types and used default arguments.
// expect exit 0
template<class T> constexpr int choose(T) { return 1; }
template<class T> constexpr int choose(T*) { return 2; }
template<class T> constexpr int choose(const T*) { return 3; }
static_assert(choose((int*)0) == 2, "int pointer");
static_assert(choose((long*)0) == 2, "long pointer");
static_assert(choose((const int*)0) == 3, "const pointer");
int (*address)(int*) = choose;
template<class T> constexpr int other(T, int = 0) { return 4; }
template<class T> constexpr int other(T*, int = 0) { return 5; }
static_assert(other((int*)0) == 5, "one argument");
static_assert(other((long*)0, 1) == 5, "two arguments");
int main() { return address((int*)0) == 2 ? 0 : 1; }
