template<int N> struct A {
  using U = unsigned _BitInt(N);
  typedef const _BitInt(N+1) S;
  U *get();
};
template<int N> int f(){ return sizeof(unsigned _BitInt(N+2)); }
int main(){return 0;}

// A type transform followed by initialization retains a functional cast.
template<int N> int cast_forms() {
  auto a = _BitInt(N)(0);
  auto b = _BitInt(N){0};
  return 0;
}
