// N3485 3.4.1/11p1 (diagnosable rules): the body of an unused inline member
// function is still analyzed; undeclared name and private access must be
// diagnosed; expect compile REJECT
class C { int c; };
struct U {
  int f(const C& z) const { return z.c; }
  int g() const { return undeclared_zz; }
};
int main() { return 0; }
