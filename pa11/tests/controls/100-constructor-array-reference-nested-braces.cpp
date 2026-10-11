// N3485 8.5.4p3, 13.3.1.7, 13.3.3.1.5: K5 k{{9, 4}} calls K5(const int (&)[2])
// with the inner list initializing the array temporary; expect exit 0
struct K5 { int s; K5(const int (&a)[2]) : s(a[0] * 10 + a[1]) {} };
int main() { K5 k{{9, 4}}; return k.s == 94 ? 0 : 1; }
