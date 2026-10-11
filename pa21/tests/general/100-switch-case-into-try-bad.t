// N3485 15p2: a switch must not transfer control into a try block or handler
// (case label inside try). Must reject.
int g(int k) { switch (k) { case 0: try { case 1: ++k; } catch (...) {} } return k; }
int main() { return g(0); }
