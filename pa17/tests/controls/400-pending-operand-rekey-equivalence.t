// expect exit 0
// Alias operands substituted with dependent arguments share written operand identity.
template<int> struct number {};
template<int A, int B> using difference = number<A - B>;
template<int N> int forward(difference<N, 1>*);
template<int N> int forward(number<N - 1>*) { return N; }
template<int N> int reverse(number<N - 2>*);
template<int N> int reverse(difference<N, 2>*) { return N; }
template<class T> using plus = decltype(T() + 1);
template<class U> int typed(plus<U>*);
template<class U> int typed(decltype(U() + 1)*) { return 3; }
int main() {
  return forward<3>(0) + reverse<4>(0) + typed<int>(0) - 10;
}
