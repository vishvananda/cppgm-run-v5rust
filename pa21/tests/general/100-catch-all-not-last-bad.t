// N3485 15.3p5: a catch(...) handler shall be the last handler of its try
// block. Must reject.
int main() { try { throw 1; } catch (...) { return 1; } catch (int) { return 2; } return 0; }
