// expect exit 0
int calls;
struct S { static int get() { return 7; } };
S object;
S* source() { ++calls; return &object; }
int invoke(int (*function)()) { return function(); }
int main() {
    S local;
    return invoke(local.get) == 7 && invoke(source()->get) == 7 && calls == 1 ? 0 : 1;
}
