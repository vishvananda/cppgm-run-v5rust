// N3485 8.5.4p4 + 8.5.1p2: aggregate elements are initialized in order, so a
// later clause may read an earlier initialized element (arr[0].a == 5);
// expect exit 0
struct P { int a; int b; };
int main() { P arr[1] = { { 5, arr[0].a + 1 } }; return arr[0].b == 6 ? 0 : 1; }
