// expect exit 0
// An invalid unused alias remains undemanded; failed trials do not poison valid ones.
template<bool> struct enable;
template<> struct enable<true> { typedef int type; };
template<class T> using check = typename enable<T::value>::type;
struct good { static const bool value = true; };
struct bad {};
template<class C, class T> using missing = decltype(C::missing + T());
template<class C> struct outer {
  template<class T> using unused = missing<C, T>;
  template<class U, class = check<U>> static int call(int) { return 1; }
  template<class> static int call(...) { return 0; }
};
outer<bad> value;
int main() {
  return outer<bad>::call<bad>(0) + outer<bad>::call<good>(0)
       + outer<bad>::call<good>(0) - 2;
}
