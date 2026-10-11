// N3485 7p1: a simple-declaration may declare an enumeration and variables of
// it at once: enum E { A, B } e = B; (full program: exit 1)
enum E { A, B } e = B;
int main() { return e; } // 1
