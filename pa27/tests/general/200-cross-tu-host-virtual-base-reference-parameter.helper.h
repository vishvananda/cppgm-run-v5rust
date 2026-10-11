struct Base { int member; };
struct Derived : virtual Base {};
int read_value(Derived &);
