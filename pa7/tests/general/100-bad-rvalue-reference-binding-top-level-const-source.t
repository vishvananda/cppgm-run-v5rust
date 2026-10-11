const int&& source();
void sink(int&& x) {}
int f() { sink(source()); return 0; }
