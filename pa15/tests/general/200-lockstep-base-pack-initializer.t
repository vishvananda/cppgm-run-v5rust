template<int I> struct tag {};
template<int I> struct base { base(tag<I>, void*) {} };
template<int... I> struct tuple : base<I>... {
  tuple() : base<I>(tag<I>(), 0)... {}
};
int main() { tuple<0, 1> value; }
