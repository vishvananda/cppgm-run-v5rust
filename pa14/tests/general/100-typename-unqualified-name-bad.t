// N3485 14.6p5: typename shall only be applied to qualified names; expect
// reject
template <class T> struct S { typename T x; };
int main() { return 0; }
