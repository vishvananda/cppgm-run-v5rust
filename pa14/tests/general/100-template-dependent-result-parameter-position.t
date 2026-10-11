// Renaming preserves parameter identity; exchanging positions does not.
template<class T, class U> auto result(T first, U second) -> decltype(first);
template<class A, class B> auto result(A first, B second) -> decltype(second);
int main() { return sizeof(result(1, 2L)); }
