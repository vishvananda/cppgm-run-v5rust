namespace outer {
namespace {
int g(int x) { return x; }
}
namespace {
int h(int y) { return g(y); }
}
}

int f() { return outer::h(0); }
