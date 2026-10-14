// expect exit 0
int events;
int next() { events = events * 10 + 2; return 5; }
int read() { return 7; }
int read(int x) { return x; }
struct Item {
    int value;
    int &reference;
    Item(int &r, int (*fn)(), int n = next()) : value(fn() + n), reference(r) {
        events = events * 10 + 3;
    }
    Item(int &r, double n) : value(n), reference(r) {}
};
struct Scalar {
    int value;
    Scalar(int n) : value(n) {}
};
int main() {
    int source = 9;
    Item first(source, read);
    Item second{source, 4.0};
    Scalar third({6});
    first.reference = 11;
    return events == 23 && first.value == 12 && second.value == 4 &&
           third.value == 6 && source == 11 ? 0 : 1;
}
