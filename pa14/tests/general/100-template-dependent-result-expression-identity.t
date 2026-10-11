// Sharing a dependent lookup root does not make different expressions equivalent.
template<class T> T& choose(T&);
template<class T> auto result(T& value) -> decltype(choose<T>(value));
template<class U> auto result(U& renamed) -> decltype((choose<U>(renamed), U()));
int main() { int value = 0; return sizeof(result(value)); }
