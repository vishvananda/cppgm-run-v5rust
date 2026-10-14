// N3485 6.4.2p2: a switch condition of class type with a single non-explicit
// conversion function to an integral type is converted by calling it;
// expect exit 0
struct Y { int v; operator int() const { return v; } };
int main() { Y y = { 3 }; switch (y) { case 3: return 0; default: return 1; } }
