// N3485 11p4, 14.6.2: type arguments check every qualifier on replay; expect reject
struct Base {
private:
  struct Nested { typedef int type; };
};
template<class T> struct Holder {};
template<class T> struct Import {
  using typename Holder<typename T::Nested::type>::type;
};
Import<Base> value;
