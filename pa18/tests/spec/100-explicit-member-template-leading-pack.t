template<class...> struct list {};
template<class, class> struct check;
template<class U> struct check<list<int, float>, U> { static const bool value = true; };
template<class A, class B> struct both { static const bool value = A::value && B::value; };
template<bool> struct if_c {};
template<> struct if_c<true> { typedef void type; };
template<class C> using if_ = typename if_c<C::value>::type;
template<class... T> struct box {
  template<class... U,
           class = if_<both<check<list<T...>, U>...> > >
  void subset() {}
};
int main() { box<int, float>().subset<float, int>(); }
