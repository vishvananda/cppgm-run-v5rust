// expect exit 0
struct S { int x; int f() { return 5; } };
struct H { int S::*p; char c; };
struct F { char c; int (S::*f)(); int S::*p; };
H data[3] = {{&S::x, 1}};
F functions[3] = {{1, &S::f, &S::x}};
int main() {
    S s = {5};
    return (s.*functions[0].f)() - 5 || data[1].p != nullptr ||
        data[2].p != nullptr || functions[1].f != nullptr ||
        functions[2].p != nullptr;
}
