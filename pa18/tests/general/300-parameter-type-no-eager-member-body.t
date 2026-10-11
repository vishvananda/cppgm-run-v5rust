template<class T> struct Box { T val; static T bad() { return T::missing_name; } };
void f(Box<int>);
