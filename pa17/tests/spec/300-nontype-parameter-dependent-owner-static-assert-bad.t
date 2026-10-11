// N3485 focus: 14.7.1 [temp.inst] nested type lookup instantiates class assertions.
template<class T> struct hard {
  static_assert(T::value, "hard error");
  typedef bool type;
};
struct bad { static constexpr bool value = false; };
template<class T, typename hard<T>::type = true> void use(T) {}
void test() { use(bad()); }
