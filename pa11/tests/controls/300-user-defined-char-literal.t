// N3485 2.14.8p1/p6, 13.5.8: a user-defined-character-literal 'a'_c calls
// operator"" _c(char); expect exit 0
int operator"" _c(char c) { return c + 1; }
int main() { return 'a'_c == 'b' ? 0 : 1; }
