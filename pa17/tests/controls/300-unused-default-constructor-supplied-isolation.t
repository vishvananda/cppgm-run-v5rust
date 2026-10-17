// expect exit 0
struct Cell {
    template<class T = int> Cell(int x = T::missing) {}
};
struct Unused { Cell cells[2]; };
struct Supplied : Cell { Supplied(int n) : Cell(n) {} };
struct Inherited : Cell { using Cell::Cell; };
inline void unused() { Cell c; Unused u; Inherited i; }
int main() { Cell c(3); Supplied s(4); Inherited i(5); return 0; }
