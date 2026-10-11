// N3485 5.3.1p5, 3.9p5: the address of an object of incomplete class type can
// be taken (extern O b; &b), the class completed later; expect exit 0
struct O;
extern O b;
O* g = &b;
struct O { int x; };
O b;
int main() { return g == &b ? 0 : 1; }
