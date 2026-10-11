struct A { virtual ~A() = 0; };
struct B { virtual ~B(); };
struct C : A, B { ~C(); };

void take(C);
void pass(C & value) { take(value); }
