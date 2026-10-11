namespace N { struct B {}; }
struct C : N::B { C() : N::B() {} };
