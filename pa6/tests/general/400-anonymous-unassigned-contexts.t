void f(struct { int a; });
int sizes[sizeof(struct { int a; })];
struct { int a; } result() { return {}; }
void g() { (struct {} *)0; new struct { int a; }; }
