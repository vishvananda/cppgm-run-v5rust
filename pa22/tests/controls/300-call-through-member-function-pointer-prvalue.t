// N3485 5.5p2-p3, 5.2.2p1: x.*choose() where choose() returns a pointer to
// member function (a class-type-sized prvalue) and the result is called;
// expect exit 0
struct D { int d; int fd() const { return d; } };
typedef int (D::*DF)() const;
DF choose() { return &D::fd; }
int main() { D x; x.d = 4; return (x.*choose())() == 4 ? 0 : 1; }
