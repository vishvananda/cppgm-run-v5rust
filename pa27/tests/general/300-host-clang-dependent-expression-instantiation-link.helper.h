// Function templates whose instantiations the C unit defines under clang's
// names; the course unit only declares them, so its calls name those symbols.
template<bool B, class T = void> struct enable_if {};
template<class T> struct enable_if<true, T> { typedef T type; };
template<class T> struct is_integral { static const bool value = false; };
template<> struct is_integral<int> { static const bool value = true; };
template<class T> struct box { T value; T get() const { return value; } };

template<class T> auto unbox(const box<T>& b) -> decltype(b.get());
template<class T>
typename enable_if<is_integral<T>::value && sizeof(T) == 4, T>::type twice(T value);
template<class T, int N> int last(T (&values)[N - 1], int (&)[sizeof(T) * N]);
