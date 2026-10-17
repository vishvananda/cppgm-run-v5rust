// expect reject
// Preserve the frozen compiler's lowering rejection for this selected parameter.
struct Cell {
    Cell& operator=(int) volatile;
    operator int() const volatile { return 4; }
};
struct Store { volatile Cell cell; };
int main() { Store a{}, b{}; a = b; }
