// expect reject
struct V { V(int); };
struct B : virtual V { B() : V(1) {} };
struct M { private: M(); };
struct D : B { M m; D() = default; };
void use() { D x; }
