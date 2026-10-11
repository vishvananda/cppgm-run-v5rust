// N3485 8.5.3/5: const long& bound to the int literal 5 binds to a temporary
// long initialized from 5; expect exit 0
int main() { const long &w = 5; const double &d = 2; return w == 5 && d == 2 ? 0 : 1; }
