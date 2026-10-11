// VALIDATION: compile-pass
// N3485 14.8.2.1: a function-template overload set is non-deduced.
namespace source {
template<class T> T & function(T & value) { return value; }
}

template<class T> struct trap {
  static_assert(sizeof(T) == 0, "must not instantiate");
  typedef int type;
};

template<class T> typename trap<T>::type select(T const &);
int select(int & (*)(int &)) { return 0; }

int main() { return select(source::function); }
