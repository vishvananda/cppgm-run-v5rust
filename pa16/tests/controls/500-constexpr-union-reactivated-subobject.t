// N3485 5.19p2, 9.5p1: starting a variant's lifetime does not restore its
// formerly initialized scalar subobjects; expect reject
struct Pair { int x; int y; };
union U { Pair a; Pair b; };
constexpr int read() {
    U u = {{2, 3}};
    u.b.x = 7;
    u.a.x = 9;
    return u.a.y;
}
constexpr int n = read();
int main() { return n; }
