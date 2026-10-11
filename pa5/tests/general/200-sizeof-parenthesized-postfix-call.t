// N3485 5.3p1: sizeof (g)(1) is sizeof applied to the unary-expression (g)(1);
// the call binds first (full program: exit 4)
int g(int) { return 0; }
int main() { return sizeof (g)(1); } // 4
