void h() {}
void g() { return h(); }
int main() { g(); return 0; }
