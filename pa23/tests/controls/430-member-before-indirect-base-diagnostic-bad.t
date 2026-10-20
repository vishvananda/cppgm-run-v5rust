// expect reject
struct V { V(int); };
struct B : virtual V { B() : V(1) {} };
struct D : B { int& r; D() {} };
void use() { D x; }
