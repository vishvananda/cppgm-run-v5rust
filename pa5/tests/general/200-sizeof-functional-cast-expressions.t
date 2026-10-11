// N3485 5.3.3p1, 8.2p2: sizeof(int(1)) and sizeof(C{}) apply sizeof to
// expressions, which cannot be type-ids (full program: exit 5)
struct C {};
int main() { return sizeof(int(1)) + sizeof(C{}); } // 5
