// N3485 9.6p4: an enumerator stored in a bit-field of its enum type wide
// enough for all its values compares equal (E{0..3} in 2 bits reads back 3);
// expect exit 0
enum E { A, B, C, D };
struct S { E e : 2; };
int main() { S s; s.e = D; return s.e == D ? 0 : 1; }
