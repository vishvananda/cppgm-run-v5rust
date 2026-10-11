// N3485 3.3.1p4, 7.2p10: an unscoped enumerator a declared in the scope that
// already has variable a is a conflicting declaration; must be rejected
int a = 5;
enum E : int { a = 1 };
int main() { return a; }
