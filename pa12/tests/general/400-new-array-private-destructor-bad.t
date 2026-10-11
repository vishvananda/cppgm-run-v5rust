// AUDIT-ID: NEW-ARRAY-DTOR-ACCESS
// AUDIT-EXPECT: reject
struct Hidden { int value; private: ~Hidden() {} };
void f() { new Hidden[2]{}; }
