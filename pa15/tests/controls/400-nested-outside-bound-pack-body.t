// A demanded body follows each copied member's source link to its definition. expect exit 0
int sum(int a, int b) { return a + b; }
template<class... T> struct Outer {
  template<class... U> struct Inner {
    static int run();
    static int data[sizeof...(T)];
  };
};
template<class... A> template<class... B>
int Outer<A...>::Inner<B...>::run() {
  static_assert(sizeof...(A) == 2 && sizeof...(B) == 2, "bound outside frames");
  return sum((sizeof(A) + sizeof(B))...);
}
template<class... A> template<class... B>
int Outer<A...>::Inner<B...>::data[] = {(sizeof(A) + sizeof(B))...};
int main() {
  return Outer<char, short>::Inner<int, long>::run() - 15
       + Outer<char, short>::Inner<int, long>::data[0] - 5;
}
