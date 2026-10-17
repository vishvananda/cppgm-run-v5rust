// expect exit 0
int seed() { return 7; }
struct Cell {
    int n;
    template<class T = int> Cell(int x = seed()) : n(x) {}
};
struct Inherited : Cell { using Cell::Cell; };
struct Store { Cell cells[9]; };
inline void unused() { Cell c; Inherited i; Store s; }
int main() {
    Cell c;
    Inherited i;
    Store s[2];
    return c.n + i.n + s[1].cells[8].n - 21;
}
