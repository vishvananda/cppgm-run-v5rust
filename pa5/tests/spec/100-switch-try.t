// N3485 focus: 6.4.2 [stmt.switch], 6.5 [stmt.iter], 15.3 [except.handle]
int main() { switch (x) { case 1: break; default: goto done; } do x = x - 1; while (x); try { throw x; } catch (...) { continue; } done: return 0; }
