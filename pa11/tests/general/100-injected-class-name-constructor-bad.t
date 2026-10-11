// N3485 3.4.3.1p2: in K::K the name K::K designates the constructor, not a
// type, so K::K* is not a type-id. Must reject.
struct K { int x; };
int main() { return int(sizeof(K::K*)); }
