template<bool, class = void> struct enable {};
template<class T> struct enable<true, T> { typedef T type; };
struct empty {};
template<class, class = void> struct select { enum { value = 1 }; };
template<class T>
struct select<T, typename enable<!__is_empty(T)>::type> {
  enum { value = 2 };
};
static_assert(select<empty>::value == 1, "");
int main() { return 0; }
