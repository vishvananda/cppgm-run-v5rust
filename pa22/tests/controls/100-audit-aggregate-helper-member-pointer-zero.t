// expect exit 0
// Omitted helper members use language null values, including data offset -1.
struct A { int n; int f() { return n; } };
struct Data { int n; int A::* p; };
struct Function { int n; int (A::* p)(); };
Data partial(int n) { return Data{n}; }
Data full(int n) { return Data{n, &A::n}; }
Function partial_function(int n) { return Function{n}; }
int main() {
  A a{7};
  Data x = partial(3), y = full(5);
  Data array[2] = {{1, &A::n}, {}};
  Function z = partial_function(9);
  return x.n == 3 && x.p == nullptr && y.n == 5 && a.*y.p == 7
      && array[0].n == 1 && a.*array[0].p == 7 && array[1].n == 0
      && array[1].p == nullptr && z.n == 9 && z.p == nullptr ? 0 : 1;
}
