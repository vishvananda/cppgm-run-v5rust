// N3485 8.3.5/3.3.4: in int (*get(int i))(int) {... } the parameters of the
// function are those of the declarator get(int i), visible in the body;
// expect exit 0
static int lt(int x) { return x + 1; }
int (*get(int i))(int) { return i ? lt : 0; }
int main() { return get(1)(-1); }
