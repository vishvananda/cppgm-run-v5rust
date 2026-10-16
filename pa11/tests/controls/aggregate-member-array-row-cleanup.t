// expect exit 0
// Each row of a counted member array owns its partial construction cleanup.
int made;
int destroyed;

struct Element {
    int value;
    Element(int input) : value(input) { made = made * 10 + input; }
    ~Element() { destroyed = destroyed * 10 + value; }
};

int main() {
    {
        struct Grid { Element cells[2][2]; };
        Grid grid = {{{1, 2}, {3, 4}}};
        if (made != 1234 || grid.cells[1][0].value != 3)
            return 1;
    }
    return destroyed == 4321 ? 0 : 2;
}
