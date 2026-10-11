// N3485 14.7.1: member lookup instantiates the class specialization.
template<int N> struct A {
  static_assert(N == 0, "bad");
  typedef int type;
};
typedef A<1>::type result;
