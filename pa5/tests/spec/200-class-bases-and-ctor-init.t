// N3485 focus: 10 [class.derived], 12.6.2 [class.base.init]
struct B1 {};
struct B2 {};
struct C : virtual public B1, protected B2 { int x; C() : B1(), B2{}, x{1} {} };
