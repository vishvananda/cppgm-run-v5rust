// N3485 8.5.2p1, 5.1.1p6: a char array initialized by a parenthesized string
// literal in braces ({("ab")}) - both hosts accept even with -pedantic-errors;
// expect exit 0
struct D { char d[4]; };
int main() { D dd = {("pq")}; char a[3] = {("ab")}; return dd.d[1] == 'q' && a[1] == 'b' ? 0 : 1; }
