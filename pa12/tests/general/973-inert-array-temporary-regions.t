typedef int A[2];
int calls;
int destroyed;
int one() { ++calls; return 1; }
int two() { ++calls; return 2; }
struct T { T() {} ~T() { ++destroyed; } };
int use(const A& a) { return a[0] + a[1]; }
int with_calls() { T t; return use(A{one(), two()}); }
int with_literals() { T t; return use(A{1, 2}); }
int with_reference() { T t; const A& a = A{one(), two()}; return use(a); }
int main() {
  return with_calls() == 3 && with_literals() == 3 && with_reference() == 3
    && calls == 4 && destroyed == 3 ? 0 : 1;
}
