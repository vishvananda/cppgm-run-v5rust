// Function templates whose signatures depend on expressions the course
// keeps in their dependent form (Itanium 5.1.5, `DT`, `X..E`, `A..._`).
template<bool B, class T = void> struct enable_if {};
template<class T> struct enable_if<true, T> { typedef T type; };
template<class T> struct is_pointer { static const bool value = false; };
template<class T> struct is_pointer<T*> { static const bool value = true; };
template<class T> T&& declval();
template<class T> struct traits { static const int bits = 8 * sizeof(T); };

template<class T> struct holder
{
  int get() const { return int(sizeof(T)); }
};

struct runner { int run() { return 3; } };

template<class T> auto measure(T& value) -> decltype(holder<T>().get());
template<class T>
typename enable_if<(sizeof(T) > 2) && !is_pointer<T>::value, int>::type wide(T value);
template<class T> int fill(T* first, int (&counts)[sizeof(T) + 1]);
template<class T>
typename enable_if<traits<unsigned>::bits == 32, T>::type checked(T value);
template<class T> auto ran(T& value) -> decltype(declval<T>().run(), int());
