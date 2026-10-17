// N3485 7.1.5p4: a constexpr constructor of a nonempty union must initialize
// a member even when the constructor is never called; expect reject
union U { int x; float y; constexpr U() {} };
int main() { return 0; }
