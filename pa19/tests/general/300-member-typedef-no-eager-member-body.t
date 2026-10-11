template<class T> struct Trap { static T bad() { return T::missing_name; } typedef T type; };
Trap<int>::type *f();
