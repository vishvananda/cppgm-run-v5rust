// defining TU of Base/Derived
struct Base { virtual ~Base(); virtual int get() const; int b; };
struct Derived : virtual Base { Derived(); virtual int get() const; int d; };
Base::~Base() {} int Base::get() const { return 1; }
Derived::Derived() : d(7) {} int Derived::get() const { return d + 40; }
