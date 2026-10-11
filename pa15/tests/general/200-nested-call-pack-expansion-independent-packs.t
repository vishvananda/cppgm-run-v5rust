template<class...> struct S {};
int sum(int a, int b) { return a + b; }
template<class U, class... T> int get(T...) { return sizeof(U); }
template<class... U, class... T> int transform(S<U...>, T... t) { return sum(get<U>(t...)...); }
// Keep both pack partitions observable in an emitted address, including at O2.
int (*transform_address)(S<char, short>, int) = &transform<char, short>;
int main() { return transform(S<char, short>(), 4) - 3 ||
                    transform_address(S<char, short>(), 4) != 3; }
