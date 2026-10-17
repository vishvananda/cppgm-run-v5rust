// expect reject
struct Cell {
    template<class T = int> Cell(int x = T::missing) {}
};
struct Store { Cell cells[2]; };
inline void unused() { Store s; }
int main() { Store s; }
