// N3485 11p4, 14.6.2: every nested qualifier is checked on replay; expect reject
struct Base {
private:
  struct Nested { typedef int type; };
};
template<class T> struct Import : T { using typename T::Nested::type; };
Import<Base> value;
