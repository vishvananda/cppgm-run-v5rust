struct Leaf { int value; };
struct Branch { Leaf leaves[2]; };
inline int ignored() { Branch unused = {}; return 1; }
inline int demanded() { Branch unused = {{{3}, {4}}}; return 2; }
struct Opaque;
extern Opaque object;
int main() { Branch unused = {}; return demanded() + (&object != 0); }
